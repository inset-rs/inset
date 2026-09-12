//! Flutter counterpart: `widgets/sliver.dart`.
//!
//! The lazily built slivers and the element that manages their children. `SliverGrid`,
//! `SliverOpacity`, `SliverIgnorePointer`, `SliverOffstage`, `SliverConstrainedCrossAxis`,
//! `SliverCrossAxisGroup`, `SliverMainAxisGroup` and `SliverEnsureSemantics` wait for their
//! render objects; see `PORTING.md`. `SliverToBoxAdapter` and `SliverPadding` are Flutter's
//! `basic.dart` and live in `widgets/basic.rs`.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_foundation::{App, Handle};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, ItemExtentBuilder, KeepAliveParentData, RenderHandle,
    RenderSliver, RenderSliverBoxChildManager, RenderSliverFixedExtentList, RenderSliverList,
    RenderSliverMultiBoxAdaptor, RenderSliverVariedExtentList, RenderSliverWithKeepAliveMixin,
    SliverConstraints, SliverMultiBoxAdaptorParentData,
};

use crate::framework::{
    AnyElement, BuildContext, Element, ElementData, IntoWidget, KeyRef, ParentDataWidget,
    RenderObjectElement, RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget,
    Slot, Widget, WidgetKind, WidgetRef, downcast_widget,
};
use crate::widgets::scroll_delegate::{
    ChildIndexGetter, NullableIndexedWidgetBuilder, SemanticIndexCallback,
    SliverChildBuilderDelegate, SliverChildDelegateRef, SliverChildListDelegate, same_delegate,
};

// ---------------------------------------------------------------------------------------------
// The widget kinds

/// A base class for slivers that have [`KeepAlive`] children.
///
/// See also:
///
/// * [`KeepAlive`], which marks whether its child widget should be kept alive.
/// * `SliverChildBuilderDelegate` and `SliverChildListDelegate`, slivers which make use of the
///   keep alive functionality through the `addAutomaticKeepAlives` property.
/// * [`SliverList`], a sliver widget that is commonly wrapped with [`KeepAlive`] widgets to
///   preserve its sliver child subtrees.
pub trait SliverWithKeepAliveWidget: RenderObjectWidget
where
    Self::RenderObject: RenderSliverWithKeepAliveMixin,
{
}

/// A base class for slivers that have multiple box children.
///
/// Helps implementors build their children lazily using a `SliverChildDelegate`.
///
/// The widgets returned by the [`delegate`](Self::delegate) are cached and the delegate is only
/// consulted again if it changes and the new delegate's `SliverChildDelegate::should_rebuild`
/// method returns true.
pub trait SliverMultiBoxAdaptorWidget: SliverWithKeepAliveWidget
where
    Self::RenderObject: RenderSliverMultiBoxAdaptor,
{
    /// The delegate that provides the children for this widget.
    ///
    /// The children are constructed lazily using this delegate to avoid creating more children
    /// than are visible through the `Viewport`.
    ///
    /// ## Using more than one delegate in a `Viewport`
    ///
    /// If multiple delegates are used in a single scroll view, the first child of each delegate
    /// will always be laid out, even if it extends beyond the currently viewable area. This is
    /// because at least one child is required in order to estimate the max scroll offset for the
    /// whole scroll view, as it uses the currently built children to estimate the remaining
    /// children's extent.
    fn delegate(&self) -> &SliverChildDelegateRef;

    /// Creates the element that lazily builds this widget's children.
    ///
    /// Dart's `createElement`; an implementor that needs the moved children replaced passes
    /// `true` (see [`SliverMultiBoxAdaptorElement::create`]).
    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement
    where
        Self: Sized,
    {
        SliverMultiBoxAdaptorElement::<Self>::create(app, this, false).as_element()
    }

    /// Returns an estimate of the max scroll extent for all the children.
    ///
    /// Implementors should override this function if they have additional information about
    /// their max scroll extent.
    ///
    /// This is used by [`SliverMultiBoxAdaptorElement`] to implement part of the
    /// `RenderSliverBoxChildManager` API.
    ///
    /// The default implementation defers to the [`delegate`](Self::delegate) via its
    /// `SliverChildDelegate::estimate_max_scroll_offset` method.
    fn estimate_max_scroll_offset(
        &self,
        constraints: Option<SliverConstraints>,
        first_index: i32,
        last_index: i32,
        leading_scroll_offset: f64,
        trailing_scroll_offset: f64,
    ) -> Option<f64> {
        let _ = constraints;
        debug_assert!(last_index >= first_index);
        self.delegate().estimate_max_scroll_offset(
            first_index,
            last_index,
            leading_scroll_offset,
            trailing_scroll_offset,
        )
    }
}

/// The kind tag of [`IntoWidget`] for a [`SliverMultiBoxAdaptorWidget`].
pub struct SliverMultiBoxAdaptorKind;

/// The erased form of a [`SliverMultiBoxAdaptorWidget`].
pub struct SliverMultiBoxAdaptor<W: SliverMultiBoxAdaptorWidget>(pub W)
where
    W::RenderObject: RenderSliverMultiBoxAdaptor;

impl<W: SliverMultiBoxAdaptorWidget> IntoWidget<SliverMultiBoxAdaptorKind> for W
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    fn into_widget(self) -> WidgetRef {
        Rc::new(SliverMultiBoxAdaptor(self))
    }
}

impl<W: SliverMultiBoxAdaptorWidget> Widget for SliverMultiBoxAdaptor<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    fn key(&self) -> Option<&KeyRef> {
        RenderObjectWidget::key(&self.0)
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        SliverMultiBoxAdaptorWidget::create_element(&self.0, app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: SliverMultiBoxAdaptorWidget> Debug for SliverMultiBoxAdaptor<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------------------------
// SliverList

/// A sliver that places multiple box children in a linear array along the main axis.
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
///
/// Each child is forced to have the `SliverConstraints::cross_axis_extent` in the cross axis but
/// determines its own main axis extent.
///
/// [`SliverList`] determines its scroll offset by "dead reckoning" because children outside the
/// visible part of the sliver are not materialized, which means [`SliverList`] cannot learn their
/// main axis extent. Instead, newly materialized children are placed adjacent to existing
/// children.
///
/// If the children have a fixed extent in the main axis, consider using
/// [`SliverFixedExtentList`] rather than [`SliverList`] because [`SliverFixedExtentList`] does
/// not need to perform layout on its children to obtain their extent in the main axis and is
/// therefore more efficient.
///
/// See also:
///
///  * `CustomScrollView`, which accepts slivers like [`SliverList`] to create custom scroll
///    effects.
///  * [`SliverFixedExtentList`], which is more efficient for children with the same extent in
///    the main axis.
///  * `SliverGrid`, which places multiple children in a two dimensional grid.
#[derive(Debug)]
pub struct SliverList {
    pub key: Option<KeyRef>,
    /// The delegate that provides the children for this widget.
    pub delegate: SliverChildDelegateRef,
}

impl SliverList {
    /// Creates a sliver that places box children in a linear array.
    pub fn new(delegate: SliverChildDelegateRef) -> SliverList {
        SliverList {
            key: None,
            delegate,
        }
    }

    /// A sliver that places multiple box children in a linear array along the main axis.
    ///
    /// This constructor is appropriate for sliver lists with a large (or infinite) number of
    /// children because the builder is called only for those children that are actually visible.
    ///
    /// Providing a non-`None` [`item_count`](SliverListBuilder::item_count) improves the ability
    /// of the [`SliverList`] to estimate the maximum scroll extent.
    ///
    /// The item builder will be called only with indices greater than or equal to zero and less
    /// than the item count.
    pub fn builder(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
    ) -> SliverListBuilder {
        SliverListBuilder::new(Rc::new(item_builder))
    }

    /// A sliver that places multiple box children, separated by box widgets, in a linear array
    /// along the main axis.
    ///
    /// This constructor is appropriate for sliver lists with a large (or infinite) number of
    /// children because the builder is called only for those children that are actually visible.
    ///
    /// The `separator_builder` is similar to `item_builder`, except it is the widget that gets
    /// placed between `item_builder(context, index)` and `item_builder(context, index + 1)`.
    pub fn separated(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
        separator_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
    ) -> SliverListSeparated {
        SliverListSeparated::new(Rc::new(item_builder), Rc::new(separator_builder))
    }

    /// A sliver that places multiple box children in a linear array along the main axis, built
    /// from an explicit list of widgets.
    pub fn list(children: Vec<WidgetRef>) -> SliverListList {
        SliverListList::new(children)
    }

    /// Dart `SliverList(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverList {
        self.key = Some(key);
        self
    }
}

impl RenderObjectWidget for SliverList {
    type RenderObject = RenderSliverList;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        RenderSliverList::new(app, child_manager_of::<SliverList>(app, context)).as_object()
    }
}

impl SliverWithKeepAliveWidget for SliverList {}

impl SliverMultiBoxAdaptorWidget for SliverList {
    fn delegate(&self) -> &SliverChildDelegateRef {
        &self.delegate
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        SliverMultiBoxAdaptorElement::<SliverList>::create(app, this, true).as_element()
    }
}

/// Dart's `SliverList.builder(..)` arguments; `into_widget` builds the [`SliverList`].
pub struct SliverListBuilder {
    key: Option<KeyRef>,
    delegate: SliverChildBuilderDelegate,
}

/// The fluent `SliverChildBuilderDelegate` arguments the `builder` constructors share.
macro_rules! builder_constructor_setters {
    ($name:ident) => {
        /// Dart `key:`.
        pub fn key(mut self, key: KeyRef) -> $name {
            self.key = Some(key);
            self
        }

        /// Dart `itemCount:`.
        pub fn item_count(mut self, item_count: i32) -> $name {
            self.delegate = self.delegate.child_count(item_count);
            self
        }

        /// Dart `addAutomaticKeepAlives:`.
        pub fn add_automatic_keep_alives(mut self, value: bool) -> $name {
            self.delegate = self.delegate.add_automatic_keep_alives(value);
            self
        }

        /// Dart `addRepaintBoundaries:`.
        pub fn add_repaint_boundaries(mut self, value: bool) -> $name {
            self.delegate = self.delegate.add_repaint_boundaries(value);
            self
        }

        /// Dart `addSemanticIndexes:`.
        pub fn add_semantic_indexes(mut self, value: bool) -> $name {
            self.delegate = self.delegate.add_semantic_indexes(value);
            self
        }
    };
}

/// The fluent `SliverChildListDelegate` arguments the `list` constructors share.
macro_rules! list_constructor_setters {
    ($name:ident) => {
        /// Dart `key:`.
        pub fn key(mut self, key: KeyRef) -> $name {
            self.key = Some(key);
            self
        }

        /// Dart `addAutomaticKeepAlives:`.
        pub fn add_automatic_keep_alives(mut self, value: bool) -> $name {
            self.delegate = self.delegate.add_automatic_keep_alives(value);
            self
        }

        /// Dart `addRepaintBoundaries:`.
        pub fn add_repaint_boundaries(mut self, value: bool) -> $name {
            self.delegate = self.delegate.add_repaint_boundaries(value);
            self
        }

        /// Dart `addSemanticIndexes:`.
        pub fn add_semantic_indexes(mut self, value: bool) -> $name {
            self.delegate = self.delegate.add_semantic_indexes(value);
            self
        }
    };
}

impl SliverListBuilder {
    fn new(item_builder: NullableIndexedWidgetBuilder) -> SliverListBuilder {
        SliverListBuilder {
            key: None,
            delegate: SliverChildBuilderDelegate::from_builder(item_builder),
        }
    }

    builder_constructor_setters!(SliverListBuilder);

    /// Dart `semanticIndexOffset:`.
    pub fn semantic_index_offset(mut self, value: i32) -> SliverListBuilder {
        self.delegate = self.delegate.semantic_index_offset(value);
        self
    }

    /// Dart `findChildIndexCallback:`.
    pub fn find_child_index_callback(mut self, value: ChildIndexGetter) -> SliverListBuilder {
        self.delegate = self.delegate.find_child_index_callback(value);
        self
    }

    /// The `SliverList` this constructor describes.
    pub fn build(self) -> SliverList {
        let mut list = SliverList::new(self.delegate.into_delegate());
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverListBuilder {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// Dart's `SliverList.separated(..)` arguments; `into_widget` builds the [`SliverList`].
pub struct SliverListSeparated {
    key: Option<KeyRef>,
    item_builder: NullableIndexedWidgetBuilder,
    separator_builder: NullableIndexedWidgetBuilder,
    item_count: Option<i32>,
    find_item_index_callback: Option<ChildIndexGetter>,
    find_child_index_callback: Option<ChildIndexGetter>,
    add_automatic_keep_alives: bool,
    add_repaint_boundaries: bool,
    add_semantic_indexes: bool,
}

impl SliverListSeparated {
    fn new(
        item_builder: NullableIndexedWidgetBuilder,
        separator_builder: NullableIndexedWidgetBuilder,
    ) -> SliverListSeparated {
        SliverListSeparated {
            key: None,
            item_builder,
            separator_builder,
            item_count: None,
            find_item_index_callback: None,
            find_child_index_callback: None,
            add_automatic_keep_alives: true,
            add_repaint_boundaries: true,
            add_semantic_indexes: true,
        }
    }

    /// Dart `key:`.
    pub fn key(mut self, key: KeyRef) -> SliverListSeparated {
        self.key = Some(key);
        self
    }

    /// Dart `itemCount:`.
    pub fn item_count(mut self, item_count: i32) -> SliverListSeparated {
        self.item_count = Some(item_count);
        self
    }

    /// Dart `findItemIndexCallback:`.
    pub fn find_item_index_callback(mut self, value: ChildIndexGetter) -> SliverListSeparated {
        debug_assert!(
            self.find_child_index_callback.is_none(),
            "Cannot provide both findItemIndexCallback and findChildIndexCallback. Use \
             findItemIndexCallback as findChildIndexCallback is deprecated."
        );
        self.find_item_index_callback = Some(value);
        self
    }

    /// Dart `findChildIndexCallback:`.
    #[deprecated(
        note = "Use find_item_index_callback instead. find_child_index_callback returns child \
                indices (which include separators), while find_item_index_callback returns item \
                indices (which do not)."
    )]
    pub fn find_child_index_callback(mut self, value: ChildIndexGetter) -> SliverListSeparated {
        debug_assert!(
            self.find_item_index_callback.is_none(),
            "Cannot provide both findItemIndexCallback and findChildIndexCallback. Use \
             findItemIndexCallback as findChildIndexCallback is deprecated."
        );
        self.find_child_index_callback = Some(value);
        self
    }

    /// Dart `addAutomaticKeepAlives:`.
    pub fn add_automatic_keep_alives(mut self, value: bool) -> SliverListSeparated {
        self.add_automatic_keep_alives = value;
        self
    }

    /// Dart `addRepaintBoundaries:`.
    pub fn add_repaint_boundaries(mut self, value: bool) -> SliverListSeparated {
        self.add_repaint_boundaries = value;
        self
    }

    /// Dart `addSemanticIndexes:`.
    pub fn add_semantic_indexes(mut self, value: bool) -> SliverListSeparated {
        self.add_semantic_indexes = value;
        self
    }

    /// The `SliverList` this constructor describes.
    pub fn build(self) -> SliverList {
        let (item_builder, separator_builder) = (self.item_builder, self.separator_builder);
        let builder: NullableIndexedWidgetBuilder =
            Rc::new(move |app: &mut App, context: BuildContext, index: i32| {
                let item_index = index / 2;
                if index % 2 == 0 {
                    item_builder(app, context, item_index)
                } else {
                    let widget = separator_builder(app, context, item_index);
                    debug_assert!(widget.is_some(), "separator_builder cannot return None.");
                    widget
                }
            });
        let semantic_index_callback: SemanticIndexCallback = Rc::new(|_widget, index| {
            if index % 2 == 0 {
                Some(index / 2)
            } else {
                None
            }
        });
        let mut delegate = SliverChildBuilderDelegate::from_builder(builder)
            .add_automatic_keep_alives(self.add_automatic_keep_alives)
            .add_repaint_boundaries(self.add_repaint_boundaries)
            .add_semantic_indexes(self.add_semantic_indexes)
            .semantic_index_callback(semantic_index_callback);
        if let Some(item_count) = self.item_count {
            delegate = delegate.child_count(0.max(item_count * 2 - 1));
        }
        match (
            self.find_item_index_callback,
            self.find_child_index_callback,
        ) {
            (Some(find_item_index_callback), _) => {
                delegate = delegate.find_child_index_callback(Rc::new(move |key| {
                    find_item_index_callback(key).map(|item_index| item_index * 2)
                }));
            }
            (None, Some(find_child_index_callback)) => {
                delegate = delegate.find_child_index_callback(find_child_index_callback);
            }
            (None, None) => {}
        }
        let mut list = SliverList::new(delegate.into_delegate());
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverListSeparated {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// Dart's `SliverList.list(..)` arguments; `into_widget` builds the [`SliverList`].
pub struct SliverListList {
    key: Option<KeyRef>,
    delegate: SliverChildListDelegate,
}

impl SliverListList {
    fn new(children: Vec<WidgetRef>) -> SliverListList {
        SliverListList {
            key: None,
            delegate: SliverChildListDelegate::new(children),
        }
    }

    list_constructor_setters!(SliverListList);

    /// The `SliverList` this constructor describes.
    pub fn build(self) -> SliverList {
        let mut list = SliverList::new(self.delegate.into_delegate());
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverListList {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// SliverFixedExtentList

/// A sliver that places multiple box children with the same main axis extent in a linear array.
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
///
/// [`SliverFixedExtentList`] places its children in a linear array along the main axis starting
/// at offset zero and without gaps. Each child is forced to have the
/// [`item_extent`](Self::item_extent) in the main axis and the
/// `SliverConstraints::cross_axis_extent` in the cross axis.
///
/// [`SliverFixedExtentList`] is more efficient than [`SliverList`] because
/// [`SliverFixedExtentList`] does not need to perform layout on its children to obtain their
/// extent in the main axis.
///
/// See also:
///
///  * [`SliverVariedExtentList`], which supports children with varying (but known upfront)
///    extents.
///  * [`SliverList`], which does not require its children to have the same extent in the main
///    axis.
#[derive(Debug)]
pub struct SliverFixedExtentList {
    pub key: Option<KeyRef>,
    /// The delegate that provides the children for this widget.
    pub delegate: SliverChildDelegateRef,
    /// The extent the children are forced to have in the main axis.
    pub item_extent: f64,
}

impl SliverFixedExtentList {
    /// Creates a sliver that places box children with the same main axis extent in a linear
    /// array.
    pub fn new(delegate: SliverChildDelegateRef, item_extent: f64) -> SliverFixedExtentList {
        SliverFixedExtentList {
            key: None,
            delegate,
            item_extent,
        }
    }

    /// A sliver that places multiple box children in a linear array along the main axis, built
    /// lazily.
    ///
    /// This constructor is appropriate for sliver lists with a large (or infinite) number of
    /// children whose extent is already determined.
    pub fn builder(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
        item_extent: f64,
    ) -> SliverFixedExtentListBuilder {
        SliverFixedExtentListBuilder::new(Rc::new(item_builder), item_extent)
    }

    /// A sliver that places multiple box children in a linear array along the main axis, built
    /// from an explicit list of widgets.
    pub fn list(children: Vec<WidgetRef>, item_extent: f64) -> SliverFixedExtentListList {
        SliverFixedExtentListList::new(children, item_extent)
    }

    /// Dart `SliverFixedExtentList(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverFixedExtentList {
        self.key = Some(key);
        self
    }
}

impl RenderObjectWidget for SliverFixedExtentList {
    type RenderObject = RenderSliverFixedExtentList;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        RenderSliverFixedExtentList::new(
            app,
            child_manager_of::<SliverFixedExtentList>(app, context),
            self.item_extent,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverFixedExtentList>,
    ) {
        render_object.set_item_extent(app, self.item_extent);
    }
}

impl SliverWithKeepAliveWidget for SliverFixedExtentList {}

impl SliverMultiBoxAdaptorWidget for SliverFixedExtentList {
    fn delegate(&self) -> &SliverChildDelegateRef {
        &self.delegate
    }
}

/// Dart's `SliverFixedExtentList.builder(..)` arguments.
pub struct SliverFixedExtentListBuilder {
    key: Option<KeyRef>,
    delegate: SliverChildBuilderDelegate,
    item_extent: f64,
}

impl SliverFixedExtentListBuilder {
    fn new(
        item_builder: NullableIndexedWidgetBuilder,
        item_extent: f64,
    ) -> SliverFixedExtentListBuilder {
        SliverFixedExtentListBuilder {
            key: None,
            delegate: SliverChildBuilderDelegate::from_builder(item_builder),
            item_extent,
        }
    }

    builder_constructor_setters!(SliverFixedExtentListBuilder);

    /// Dart `semanticIndexOffset:`.
    pub fn semantic_index_offset(mut self, value: i32) -> SliverFixedExtentListBuilder {
        self.delegate = self.delegate.semantic_index_offset(value);
        self
    }

    /// Dart `findChildIndexCallback:`.
    pub fn find_child_index_callback(
        mut self,
        value: ChildIndexGetter,
    ) -> SliverFixedExtentListBuilder {
        self.delegate = self.delegate.find_child_index_callback(value);
        self
    }

    /// The `SliverFixedExtentList` this constructor describes.
    pub fn build(self) -> SliverFixedExtentList {
        let mut list = SliverFixedExtentList::new(self.delegate.into_delegate(), self.item_extent);
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverFixedExtentListBuilder {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// Dart's `SliverFixedExtentList.list(..)` arguments.
pub struct SliverFixedExtentListList {
    key: Option<KeyRef>,
    delegate: SliverChildListDelegate,
    item_extent: f64,
}

impl SliverFixedExtentListList {
    fn new(children: Vec<WidgetRef>, item_extent: f64) -> SliverFixedExtentListList {
        SliverFixedExtentListList {
            key: None,
            delegate: SliverChildListDelegate::new(children),
            item_extent,
        }
    }

    list_constructor_setters!(SliverFixedExtentListList);

    /// The `SliverFixedExtentList` this constructor describes.
    pub fn build(self) -> SliverFixedExtentList {
        let mut list = SliverFixedExtentList::new(self.delegate.into_delegate(), self.item_extent);
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverFixedExtentListList {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// SliverVariedExtentList

/// A sliver that places its box children in a linear array and constrains them to have the
/// corresponding extent returned by [`item_extent_builder`](Self::item_extent_builder).
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
///
/// [`SliverVariedExtentList`] arranges its children in a line along the main axis starting at
/// offset zero and without gaps. Each child is constrained to the corresponding extent along the
/// main axis and the `SliverConstraints::cross_axis_extent` along the cross axis.
///
/// [`SliverVariedExtentList`] is more efficient than [`SliverList`] because it does not need to
/// lay out its children to obtain their extent along the main axis. It's a little more flexible
/// than [`SliverFixedExtentList`] because this allows the children to have different extents.
pub struct SliverVariedExtentList {
    pub key: Option<KeyRef>,
    /// The delegate that provides the children for this widget.
    pub delegate: SliverChildDelegateRef,
    /// The children extent builder.
    ///
    /// Should return `None` if asked to build an item extent with a greater index than exists.
    pub item_extent_builder: ItemExtentBuilder,
}

impl SliverVariedExtentList {
    /// Creates a sliver that places box children with the corresponding main axis extent in a
    /// linear array.
    pub fn new(
        delegate: SliverChildDelegateRef,
        item_extent_builder: ItemExtentBuilder,
    ) -> SliverVariedExtentList {
        SliverVariedExtentList {
            key: None,
            delegate,
            item_extent_builder,
        }
    }

    /// A sliver that places multiple box children in a linear array along the main axis, built
    /// lazily.
    pub fn builder(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
        item_extent_builder: ItemExtentBuilder,
    ) -> SliverVariedExtentListBuilder {
        SliverVariedExtentListBuilder::new(Rc::new(item_builder), item_extent_builder)
    }

    /// A sliver that places multiple box children in a linear array along the main axis, built
    /// from an explicit list of widgets.
    pub fn list(
        children: Vec<WidgetRef>,
        item_extent_builder: ItemExtentBuilder,
    ) -> SliverVariedExtentListList {
        SliverVariedExtentListList::new(children, item_extent_builder)
    }

    /// Dart `SliverVariedExtentList(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverVariedExtentList {
        self.key = Some(key);
        self
    }
}

impl Debug for SliverVariedExtentList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SliverVariedExtentList")
            .field("key", &self.key)
            .field("delegate", &self.delegate)
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for SliverVariedExtentList {
    type RenderObject = RenderSliverVariedExtentList;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        RenderSliverVariedExtentList::new(
            app,
            child_manager_of::<SliverVariedExtentList>(app, context),
            Rc::clone(&self.item_extent_builder),
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverVariedExtentList>,
    ) {
        render_object.set_item_extent_builder(app, Rc::clone(&self.item_extent_builder));
    }
}

impl SliverWithKeepAliveWidget for SliverVariedExtentList {}

impl SliverMultiBoxAdaptorWidget for SliverVariedExtentList {
    fn delegate(&self) -> &SliverChildDelegateRef {
        &self.delegate
    }
}

/// Dart's `SliverVariedExtentList.builder(..)` arguments.
pub struct SliverVariedExtentListBuilder {
    key: Option<KeyRef>,
    delegate: SliverChildBuilderDelegate,
    item_extent_builder: ItemExtentBuilder,
}

impl SliverVariedExtentListBuilder {
    fn new(
        item_builder: NullableIndexedWidgetBuilder,
        item_extent_builder: ItemExtentBuilder,
    ) -> SliverVariedExtentListBuilder {
        SliverVariedExtentListBuilder {
            key: None,
            delegate: SliverChildBuilderDelegate::from_builder(item_builder),
            item_extent_builder,
        }
    }

    builder_constructor_setters!(SliverVariedExtentListBuilder);

    /// Dart `findChildIndexCallback:`.
    pub fn find_child_index_callback(
        mut self,
        value: ChildIndexGetter,
    ) -> SliverVariedExtentListBuilder {
        self.delegate = self.delegate.find_child_index_callback(value);
        self
    }

    /// The `SliverVariedExtentList` this constructor describes.
    pub fn build(self) -> SliverVariedExtentList {
        let mut list =
            SliverVariedExtentList::new(self.delegate.into_delegate(), self.item_extent_builder);
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverVariedExtentListBuilder {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// Dart's `SliverVariedExtentList.list(..)` arguments.
pub struct SliverVariedExtentListList {
    key: Option<KeyRef>,
    delegate: SliverChildListDelegate,
    item_extent_builder: ItemExtentBuilder,
}

impl SliverVariedExtentListList {
    fn new(
        children: Vec<WidgetRef>,
        item_extent_builder: ItemExtentBuilder,
    ) -> SliverVariedExtentListList {
        SliverVariedExtentListList {
            key: None,
            delegate: SliverChildListDelegate::new(children),
            item_extent_builder,
        }
    }

    list_constructor_setters!(SliverVariedExtentListList);

    /// The `SliverVariedExtentList` this constructor describes.
    pub fn build(self) -> SliverVariedExtentList {
        let mut list =
            SliverVariedExtentList::new(self.delegate.into_delegate(), self.item_extent_builder);
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverVariedExtentListList {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// SliverMultiBoxAdaptorElement

/// Dart's `context as SliverMultiBoxAdaptorElement`, as the render object's child manager.
fn child_manager_of<W: SliverMultiBoxAdaptorWidget>(
    app: &App,
    context: BuildContext,
) -> Rc<dyn RenderSliverBoxChildManager>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    let element = context
        .downcast::<SliverMultiBoxAdaptorElement<W>>(app)
        .expect("a SliverMultiBoxAdaptorWidget creates its render object from its own element");
    Rc::new(ChildManager::<W>(element))
}

/// An element that lazily builds children for a [`SliverMultiBoxAdaptorWidget`].
///
/// Implements `RenderSliverBoxChildManager`, which lets this element manage the children of
/// implementors of `RenderSliverMultiBoxAdaptor`. The render object holds the manager as an
/// `Rc<dyn RenderSliverBoxChildManager>`, which the widget hands it in `create_render_object`
/// after casting the [`BuildContext`] back to this element.
pub struct SliverMultiBoxAdaptorElement<W: SliverMultiBoxAdaptorWidget>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    element: ElementData,
    render_object_element: RenderObjectElementData,
    replace_moved_children: bool,
    child_elements: BTreeMap<i32, Option<AnyElement>>,
    current_before_child: Option<AnyRenderBox>,
    currently_updating_child_index: Option<i32>,
    did_underflow: bool,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: SliverMultiBoxAdaptorWidget> SliverMultiBoxAdaptorElement<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    /// Creates an element that lazily builds children for the given widget.
    ///
    /// If `replace_moved_children` is true, a new child is proactively inflated for the index
    /// that was previously occupied by a child that moved to a new index. The layout offset of
    /// the moved child is copied over to the new child. Render objects that depend on the layout
    /// offset of existing children during `RenderObject::perform_layout` should set this to true
    /// (example: `RenderSliverList`). For render objects that figure out the layout offset of
    /// their children without looking at the layout offset of existing children this should be
    /// set to false (example: `RenderSliverFixedExtentList`) to avoid inflating unnecessary
    /// children.
    pub fn create(
        app: &mut App,
        widget: WidgetRef,
        replace_moved_children: bool,
    ) -> Handle<SliverMultiBoxAdaptorElement<W>> {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(SliverMultiBoxAdaptorElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            replace_moved_children,
            child_elements: BTreeMap::new(),
            current_before_child: None,
            currently_updating_child_index: None,
            did_underflow: false,
            marker: std::marker::PhantomData,
        })
    }

    /// The render object this element manages the children of.
    pub fn render_object(self: Handle<Self>, app: &App) -> RenderHandle<W::RenderObject> {
        self.typed_render_object(app)
    }

    /// Flutter's `_build`.
    fn build(self: Handle<Self>, app: &mut App, index: i32) -> Option<WidgetRef> {
        let delegate = Rc::clone(Self::widget_of(self.as_element().widget(app)).delegate());
        delegate.build(app, self.as_element(), index)
    }

    /// `SliverMultiBoxAdaptorElement.updateChild`: preserves the old layout offset when the
    /// render object was swapped out.
    fn update_child(
        self: Handle<Self>,
        app: &mut App,
        child: Option<AnyElement>,
        new_widget: Option<WidgetRef>,
        new_slot: Option<Slot>,
    ) -> Option<AnyElement> {
        let old = Self::adaptor_parent_data(app, child)
            .map(|object| (object, Self::layout_offset(app, object)));
        let new_child = self
            .as_element()
            .update_child(app, child, new_widget, new_slot);
        let new = Self::adaptor_parent_data(app, new_child);

        // Preserve the old layoutOffset if the renderObject was swapped out.
        if let Some((old_object, layout_offset)) = old
            && let Some(new_object) = new
            && old_object != new_object
        {
            Self::set_layout_offset(app, new_object, layout_offset);
        }
        new_child
    }

    /// The child's render object, when it carries a `SliverMultiBoxAdaptorParentData`.
    fn adaptor_parent_data(app: &App, child: Option<AnyElement>) -> Option<AnyRenderObject> {
        child
            .and_then(|child| child.render_object(app))
            .filter(|object| object.parent_data_is::<SliverMultiBoxAdaptorParentData>(app))
    }

    fn layout_offset(app: &App, object: AnyRenderObject) -> Option<f64> {
        object
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
            .layout_offset()
    }

    fn set_layout_offset(app: &mut App, object: AnyRenderObject, value: Option<f64>) {
        object
            .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
            .set_layout_offset(value);
    }

    /// Flutter's `processElement` closure inside `performRebuild`.
    fn process_element(
        self: Handle<Self>,
        app: &mut App,
        index: i32,
        new_children: &BTreeMap<i32, Option<AnyElement>>,
        index_to_layout_offset: &HashMap<i32, f64>,
        children_updated: &mut bool,
    ) {
        app.get_mut(self).currently_updating_child_index = Some(index);
        let existing = app.get(self).child_elements.get(&index).copied().flatten();
        let planned = new_children.get(&index).copied().flatten();
        if existing.is_some() && existing != planned {
            // This index has an old child that isn't used anywhere and should be deactivated.
            let deactivated = self.update_child(app, existing, None, Some(Slot::Index(index)));
            app.get_mut(self).child_elements.insert(index, deactivated);
            *children_updated = true;
        }
        let built = self.build(app, index);
        let new_child = self.update_child(app, planned, built, Some(Slot::Index(index)));
        match new_child {
            Some(new_child) => {
                *children_updated = *children_updated
                    || app.get(self).child_elements.get(&index).copied().flatten()
                        != Some(new_child);
                app.get_mut(self)
                    .child_elements
                    .insert(index, Some(new_child));
                let object = new_child
                    .render_object(app)
                    .expect("a built child has a render object");
                if index == 0 {
                    Self::set_layout_offset(app, object, Some(0.0));
                } else if let Some(offset) = index_to_layout_offset.get(&index) {
                    Self::set_layout_offset(app, object, Some(*offset));
                }
                let parent_data = object.parent_data_of::<SliverMultiBoxAdaptorParentData>(app);
                if !parent_data.kept_alive() {
                    app.get_mut(self).current_before_child =
                        Some(object.as_box().expect("a sliver child is a box"));
                }
            }
            None => {
                *children_updated = true;
                app.get_mut(self).child_elements.remove(&index);
            }
        }
    }

    /// Flutter's `_extrapolateMaxScrollOffset`.
    fn extrapolate_max_scroll_offset(
        first_index: i32,
        last_index: i32,
        leading_scroll_offset: f64,
        trailing_scroll_offset: f64,
        child_count: i32,
    ) -> f64 {
        if last_index == child_count - 1 {
            return trailing_scroll_offset;
        }
        let reified_count = last_index - first_index + 1;
        let average_extent =
            (trailing_scroll_offset - leading_scroll_offset) / f64::from(reified_count);
        let remaining_count = child_count - last_index - 1;
        trailing_scroll_offset + average_extent * f64::from(remaining_count)
    }

    /// Dart's `owner!.buildScope(this, callback)`.
    fn in_build_scope(
        self: Handle<Self>,
        app: &mut App,
        callback: impl FnOnce(&mut App) + 'static,
    ) {
        let owner = self
            .as_element()
            .owner(app)
            .expect("a mounted element has an owner");
        owner.build_scope(app, self.as_element(), Some(Box::new(callback)));
    }
}

// ---- the `RenderSliverBoxChildManager` members, which Dart implements on the element ----

impl<W: SliverMultiBoxAdaptorWidget> SliverMultiBoxAdaptorElement<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    /// `RenderSliverBoxChildManager::create_child`.
    pub fn create_child(
        self: Handle<Self>,
        app: &mut App,
        index: i32,
        after: Option<AnyRenderBox>,
    ) {
        debug_assert!(app.get(self).currently_updating_child_index.is_none());
        self.in_build_scope(app, move |app| {
            let insert_first = after.is_none();
            debug_assert!(
                insert_first
                    || app
                        .get(self)
                        .child_elements
                        .get(&(index - 1))
                        .copied()
                        .flatten()
                        .is_some()
            );
            let before = if insert_first {
                None
            } else {
                app.get(self).child_elements[&(index - 1)]
                    .expect("asserted above")
                    .render_object(app)
                    .map(|object| object.as_box().expect("a sliver child is a box"))
            };
            app.get_mut(self).current_before_child = before;
            app.get_mut(self).currently_updating_child_index = Some(index);
            let built = self.build(app, index);
            let existing = app.get(self).child_elements.get(&index).copied().flatten();
            let new_child = self.update_child(app, existing, built, Some(Slot::Index(index)));
            app.get_mut(self).currently_updating_child_index = None;
            match new_child {
                Some(new_child) => {
                    app.get_mut(self)
                        .child_elements
                        .insert(index, Some(new_child));
                }
                None => {
                    app.get_mut(self).child_elements.remove(&index);
                }
            }
        });
    }

    /// `RenderSliverBoxChildManager::remove_child`.
    pub fn remove_child(self: Handle<Self>, app: &mut App, child: AnyRenderBox) {
        let index = self.render_object(app).index_of(app, child);
        debug_assert!(app.get(self).currently_updating_child_index.is_none());
        debug_assert!(index >= 0);
        self.in_build_scope(app, move |app| {
            debug_assert!(app.get(self).child_elements.contains_key(&index));
            app.get_mut(self).currently_updating_child_index = Some(index);
            let existing = app.get(self).child_elements.get(&index).copied().flatten();
            let result = self.update_child(app, existing, None, Some(Slot::Index(index)));
            debug_assert!(result.is_none());
            app.get_mut(self).currently_updating_child_index = None;
            app.get_mut(self).child_elements.remove(&index);
            debug_assert!(!app.get(self).child_elements.contains_key(&index));
        });
    }

    /// `RenderSliverBoxChildManager::estimate_max_scroll_offset`.
    pub fn estimate_max_scroll_offset(
        self: Handle<Self>,
        app: &App,
        constraints: SliverConstraints,
        first_index: Option<i32>,
        last_index: Option<i32>,
        leading_scroll_offset: Option<f64>,
        trailing_scroll_offset: Option<f64>,
    ) -> f64 {
        let Some(child_count) = self.estimated_child_count(app) else {
            return f64::INFINITY;
        };
        let first_index = first_index.expect("a finite child list reports its window");
        let last_index = last_index.expect("a finite child list reports its window");
        let leading_scroll_offset =
            leading_scroll_offset.expect("a finite child list reports its window");
        let trailing_scroll_offset =
            trailing_scroll_offset.expect("a finite child list reports its window");
        Self::widget_of(self.as_element().widget(app))
            .estimate_max_scroll_offset(
                Some(constraints),
                first_index,
                last_index,
                leading_scroll_offset,
                trailing_scroll_offset,
            )
            .unwrap_or_else(|| {
                Self::extrapolate_max_scroll_offset(
                    first_index,
                    last_index,
                    leading_scroll_offset,
                    trailing_scroll_offset,
                    child_count,
                )
            })
    }

    /// `RenderSliverBoxChildManager::estimated_child_count`.
    pub fn estimated_child_count(self: Handle<Self>, app: &App) -> Option<i32> {
        Self::widget_of(self.as_element().widget(app))
            .delegate()
            .estimated_child_count()
    }

    /// `RenderSliverBoxChildManager::child_count`.
    pub fn child_count(self: Handle<Self>, app: &mut App) -> i32 {
        if let Some(result) = self.estimated_child_count(app) {
            return result;
        }
        // Since childCount was called, we know that we reached the end of the list (as in,
        // `build` returned None once), so we know that the list is finite. Let's do an
        // open-ended binary search to find the end of the list manually.
        let mut lo = 0;
        let mut hi = 1;
        let max = i32::MAX;
        while self.build(app, hi - 1).is_some() {
            lo = hi - 1;
            if hi < max / 2 {
                hi *= 2;
            } else if hi < max {
                hi = max;
            } else {
                panic!(
                    "Could not find the number of children in {:?}.\nThe child_count getter was \
                     called (implying that the delegate's builder returned None for a positive \
                     index), but even building the child with index {hi} (the maximum possible \
                     integer) did not return None. Consider implementing estimated_child_count to \
                     avoid the cost of searching for the final child.",
                    Self::widget_of(self.as_element().widget(app)).delegate()
                );
            }
        }
        while hi - lo > 1 {
            let mid = (hi - lo) / 2 + lo;
            if self.build(app, mid - 1).is_none() {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }

    /// `RenderSliverBoxChildManager::did_start_layout`.
    pub fn did_start_layout(self: Handle<Self>, app: &App) {
        debug_assert!(self.debug_assert_child_list_locked(app));
    }

    /// `RenderSliverBoxChildManager::did_finish_layout`.
    pub fn did_finish_layout(self: Handle<Self>, app: &App) {
        debug_assert!(self.debug_assert_child_list_locked(app));
        let first_index = app
            .get(self)
            .child_elements
            .keys()
            .next()
            .copied()
            .unwrap_or(0);
        let last_index = app
            .get(self)
            .child_elements
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0);
        Self::widget_of(self.as_element().widget(app))
            .delegate()
            .did_finish_layout(first_index, last_index);
    }

    /// `RenderSliverBoxChildManager::debug_assert_child_list_locked`.
    pub fn debug_assert_child_list_locked(self: Handle<Self>, app: &App) -> bool {
        debug_assert!(app.get(self).currently_updating_child_index.is_none());
        true
    }

    /// `RenderSliverBoxChildManager::did_adopt_child`.
    pub fn did_adopt_child(self: Handle<Self>, app: &mut App, child: AnyRenderBox) {
        debug_assert!(app.get(self).currently_updating_child_index.is_some());
        let index = app.get(self).currently_updating_child_index;
        child
            .as_object()
            .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
            .index = index;
    }

    /// `RenderSliverBoxChildManager::set_did_underflow`.
    pub fn set_did_underflow(self: Handle<Self>, app: &mut App, value: bool) {
        app.get_mut(self).did_underflow = value;
    }
}

/// The element as the `Rc<dyn RenderSliverBoxChildManager>` its render object holds.
///
/// A local wrapper, because `impl RenderSliverBoxChildManager for Handle<..>` names two foreign
/// types.
struct ChildManager<W: SliverMultiBoxAdaptorWidget>(Handle<SliverMultiBoxAdaptorElement<W>>)
where
    W::RenderObject: RenderSliverMultiBoxAdaptor;

impl<W: SliverMultiBoxAdaptorWidget> RenderSliverBoxChildManager for ChildManager<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    fn create_child(&self, app: &mut App, index: i32, after: Option<AnyRenderBox>) {
        self.0.create_child(app, index, after);
    }

    fn remove_child(&self, app: &mut App, child: AnyRenderBox) {
        self.0.remove_child(app, child);
    }

    fn estimate_max_scroll_offset(
        &self,
        app: &App,
        constraints: SliverConstraints,
        first_index: Option<i32>,
        last_index: Option<i32>,
        leading_scroll_offset: Option<f64>,
        trailing_scroll_offset: Option<f64>,
    ) -> f64 {
        self.0.estimate_max_scroll_offset(
            app,
            constraints,
            first_index,
            last_index,
            leading_scroll_offset,
            trailing_scroll_offset,
        )
    }

    fn child_count(&self, app: &mut App) -> i32 {
        self.0.child_count(app)
    }

    fn estimated_child_count(&self, app: &App) -> Option<i32> {
        self.0.estimated_child_count(app)
    }

    fn did_adopt_child(&self, app: &mut App, child: AnyRenderBox) {
        self.0.did_adopt_child(app, child);
    }

    fn set_did_underflow(&self, app: &mut App, value: bool) {
        self.0.set_did_underflow(app, value);
    }

    fn did_start_layout(&self, app: &mut App) {
        self.0.did_start_layout(app);
    }

    fn did_finish_layout(&self, app: &mut App) {
        self.0.did_finish_layout(app);
    }

    fn debug_assert_child_list_locked(&self, app: &App) -> bool {
        self.0.debug_assert_child_list_locked(app)
    }
}

impl<W: SliverMultiBoxAdaptorWidget> RenderObjectElementWidget for SliverMultiBoxAdaptorElement<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    type Widget = W;

    fn widget_of(widget: &WidgetRef) -> &W {
        downcast_widget::<W>(&**widget)
            .expect("a SliverMultiBoxAdaptorElement holds its SliverMultiBoxAdaptorWidget")
    }
}

impl<W: SliverMultiBoxAdaptorWidget> RenderObjectElement for SliverMultiBoxAdaptorElement<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl<W: SliverMultiBoxAdaptorWidget> Element for SliverMultiBoxAdaptorElement<W>
where
    W::RenderObject: RenderSliverMultiBoxAdaptor,
{
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        RenderObjectElement::update(self, app, new_widget.clone());
        let new_delegate = Rc::clone(Self::widget_of(&new_widget).delegate());
        let old_delegate = Rc::clone(Self::widget_of(&old_widget).delegate());
        if !same_delegate(&new_delegate, &old_delegate)
            && (new_delegate.delegate_type() != old_delegate.delegate_type()
                || new_delegate.should_rebuild(&*old_delegate))
        {
            Element::perform_rebuild(self, app);
        }
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
        app.get_mut(self).current_before_child = None;
        let mut children_updated = false;
        debug_assert!(app.get(self).currently_updating_child_index.is_none());

        let mut new_children: BTreeMap<i32, Option<AnyElement>> = BTreeMap::new();
        let mut index_to_layout_offset: HashMap<i32, f64> = HashMap::new();
        for index in app
            .get(self)
            .child_elements
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            let child = app.get(self).child_elements[&index].expect("a live child");
            let key = child.widget(app).key().cloned();
            let new_index = key.and_then(|key| {
                Self::widget_of(self.as_element().widget(app))
                    .delegate()
                    .find_index_by_key(&key)
            });
            let child_parent_data = Self::adaptor_parent_data(app, Some(child));

            if let Some(object) = child_parent_data
                && let Some(layout_offset) = Self::layout_offset(app, object)
            {
                index_to_layout_offset.insert(index, layout_offset);
            }

            match new_index {
                Some(new_index) if new_index != index => {
                    // The layout offset of the child being moved is no longer accurate.
                    if let Some(object) = child_parent_data {
                        Self::set_layout_offset(app, object, None);
                    }
                    new_children.insert(new_index, Some(child));
                    if app.get(self).replace_moved_children {
                        // We need to make sure the original index gets processed.
                        new_children.entry(index).or_insert(None);
                    }
                    // We do not want the remapped child to get deactivated during
                    // `process_element`.
                    app.get_mut(self).child_elements.remove(&index);
                }
                _ => {
                    new_children.entry(index).or_insert(Some(child));
                }
            }
        }

        // Moving children will temporarily violate the integrity.
        self.render_object(app)
            .set_debug_child_integrity_enabled(app, false);
        for index in new_children.keys().copied().collect::<Vec<_>>() {
            self.process_element(
                app,
                index,
                &new_children,
                &index_to_layout_offset,
                &mut children_updated,
            );
        }
        // An element rebuild only updates existing children. The underflow check is here to make
        // sure we look ahead one more child if we were at the end of the child list before the
        // update. By doing so, we can update the max scroll offset during the layout phase.
        // Otherwise, the layout phase may be skipped, and the scroll view may be stuck at the
        // previous max scroll offset.
        //
        // This logic is not needed if any existing children has been updated, because we will not
        // skip the layout phase if that happens.
        if !children_updated && app.get(self).did_underflow {
            let last_key = app
                .get(self)
                .child_elements
                .keys()
                .next_back()
                .copied()
                .unwrap_or(-1);
            let right_boundary = last_key + 1;
            let existing = app
                .get(self)
                .child_elements
                .get(&right_boundary)
                .copied()
                .flatten();
            new_children.insert(right_boundary, existing);
            self.process_element(
                app,
                right_boundary,
                &new_children,
                &index_to_layout_offset,
                &mut children_updated,
            );
        }
        app.get_mut(self).currently_updating_child_index = None;
        self.render_object(app)
            .set_debug_child_integrity_enabled(app, true);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        let slot = child.slot(app).expect("a lazily built child has an index");
        let Slot::Index(index) = slot else {
            unreachable!("a lazily built child has an index slot");
        };
        debug_assert!(app.get(self).child_elements.contains_key(&index));
        app.get_mut(self).child_elements.remove(&index);
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        // The copy is so that the underlying map can be modified by the visitor.
        let children: Vec<AnyElement> = app
            .get(self)
            .child_elements
            .values()
            .map(|child| child.expect("no child slot is empty outside a rebuild"))
            .collect();
        for child in children {
            visitor(child);
        }
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        debug_assert!(
            slot == app
                .get(self)
                .currently_updating_child_index
                .map(Slot::Index)
        );
        let render_object = self.render_object(app);
        let before = app.get(self).current_before_child;
        let child = child.as_box().expect("a sliver child is a box");
        RenderSliverMultiBoxAdaptor::insert(render_object, app, child, before);
        debug_assert!(
            slot == child
                .as_object()
                .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                .index
                .map(Slot::Index)
        );
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        debug_assert!(
            new_slot
                == app
                    .get(self)
                    .currently_updating_child_index
                    .map(Slot::Index)
        );
        let render_object = self.render_object(app);
        let before = app.get(self).current_before_child;
        let child = child.as_box().expect("a sliver child is a box");
        RenderSliverMultiBoxAdaptor::move_child(render_object, app, child, before);
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        debug_assert!(app.get(self).currently_updating_child_index.is_some());
        let render_object = self.render_object(app);
        let child = child.as_box().expect("a sliver child is a box");
        RenderSliverMultiBoxAdaptor::remove(render_object, app, child);
    }
}

// ---------------------------------------------------------------------------------------------
// KeepAlive

/// Mark a child as needing to stay alive even when it's in a lazy list that would otherwise
/// remove it.
///
/// This widget is for use in a `SliverWithKeepAliveWidget` (a [`SliverMultiBoxAdaptorWidget`],
/// for instance).
///
/// See also:
///
///  * `AutomaticKeepAlive`, which allows subtrees to request to be kept alive in lazy lists.
///  * `AutomaticKeepAliveClientMixin`, which is a mixin with convenience methods for clients of
///    `AutomaticKeepAlive`. Used with `State` subclasses.
#[derive(Debug)]
pub struct KeepAlive {
    pub key: Option<KeyRef>,
    /// Whether to keep the child alive.
    ///
    /// If this is false, it is as if this widget was omitted.
    pub keep_alive: bool,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl KeepAlive {
    /// Marks a child as needing to remain alive.
    pub fn new<K>(keep_alive: bool, child: impl IntoWidget<K>) -> KeepAlive {
        KeepAlive {
            key: None,
            keep_alive,
            child: child.into_widget(),
        }
    }

    /// Dart `KeepAlive(key:)`.
    pub fn key(mut self, key: KeyRef) -> KeepAlive {
        self.key = Some(key);
        self
    }
}

impl ParentDataWidget for KeepAlive {
    type ParentData = SliverMultiBoxAdaptorParentData;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        let parent_data = render_object
            .parent_data_mut(app)
            .and_then(|parent_data| parent_data.part_mut::<KeepAliveParentData>())
            .expect("parent data does not embed a KeepAliveParentData");
        if parent_data.keep_alive != self.keep_alive {
            // No need to redo layout if it became true.
            parent_data.keep_alive = self.keep_alive;
            if !self.keep_alive
                && let Some(parent) = render_object.parent(app)
            {
                parent.mark_needs_layout(app);
            }
        }
    }

    // We only return true if `keep_alive` is true, because turning _off_ keep alive requires a
    // layout to do the garbage collection (but turning it on requires nothing, since by
    // definition the widget is already alive and won't go away _unless_ we do a layout).
    fn debug_can_apply_out_of_turn(&self) -> bool {
        self.keep_alive
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::TextDirection;
    use inset_foundation::AppCell;
    use inset_foundation::ValueKey;
    use inset_rendering::{
        ContainerRenderObjectMixin, FixedViewportOffset, RenderSliverList, ScrollCacheExtent,
        ViewportOffset,
    };

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Directionality, SizedBox};
    use crate::widgets::viewport::Viewport;

    fn key(value: u32) -> KeyRef {
        Rc::new(ValueKey::new(value)) as KeyRef
    }

    fn item(value: u32) -> WidgetRef {
        SizedBox::new().key(key(value)).height(40.0).into_widget()
    }

    fn viewport_over(sliver: WidgetRef, offset: Handle<FixedViewportOffset>) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            Viewport::new(offset.as_viewport_offset())
                .scroll_cache_extent(ScrollCacheExtent::Pixels(0.0))
                .slivers(vec![sliver]),
        )
        .into_widget()
    }

    fn first_sliver<T: inset_rendering::RenderObject>(
        harness: &Harness,
        app: &App,
    ) -> RenderHandle<T> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<inset_rendering::RenderViewport>(app)
            .expect("a RenderViewport")
            .first_child(app)
            .expect("a sliver")
            .as_object()
            .downcast::<T>(app)
            .expect("the sliver under test")
    }

    fn children_of<T>(list: RenderHandle<T>, app: &App) -> Vec<AnyRenderBox>
    where
        T: ContainerRenderObjectMixin<ChildType = AnyRenderBox>,
    {
        let mut children = Vec::new();
        let mut child = list.first_child(app);
        while let Some(current) = child {
            children.push(current);
            child = list.child_after(app, current);
        }
        children
    }

    /// A keyed reorder moves the existing children to their new indices instead of rebuilding
    /// them: `SliverChildListDelegate.findIndexByKey` finds each child's new index.
    #[test]
    fn a_keyed_reorder_keeps_the_child_render_objects() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let ordered = |values: [u32; 3]| {
            SliverList::list(values.map(item).to_vec())
                .add_repaint_boundaries(false)
                .into_widget()
        };
        let harness = Harness::mount(&mut app, viewport_over(ordered([0, 1, 2]), offset));
        harness.pump(&mut app);

        let list: RenderHandle<RenderSliverList> = first_sliver(&harness, &app);
        let before = children_of(list, &app);
        assert_eq!(before.len(), 3);

        harness.set_child(&mut app, viewport_over(ordered([2, 0, 1]), offset));
        harness.pump(&mut app);

        let after = children_of(list, &app);
        assert_eq!(after, vec![before[2], before[0], before[1]]);
    }

    /// A `KeepAlive` child that scrolls out of view moves into the sliver's keep-alive bucket
    /// instead of being destroyed.
    #[test]
    fn keep_alive_holds_a_scrolled_off_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let mut children: Vec<WidgetRef> = vec![
            KeepAlive::new(true, SizedBox::new().key(key(0)).height(40.0))
                .key(key(100))
                .into_widget(),
        ];
        children.extend((1..8).map(item));
        let sliver = SliverList::list(children)
            .add_repaint_boundaries(false)
            .into_widget();
        let harness = Harness::mount(&mut app, viewport_over(sliver, offset));
        harness.pump(&mut app);

        let list: RenderHandle<RenderSliverList> = first_sliver(&harness, &app);
        let kept = children_of(list, &app)[0];
        assert!(list.keep_alive_children(&app).is_empty());

        offset.correct_by(&mut app, 200.0);
        list.parent(&app)
            .expect("the viewport")
            .mark_needs_layout(&mut app);
        harness.pump(&mut app);

        // The first child is out of view, but it was kept alive rather than destroyed.
        assert_eq!(list.keep_alive_children(&app), vec![kept]);
        assert!(!children_of(list, &app).contains(&kept));
    }

    /// A `SliverFixedExtentList` forces its item extent on every child, whatever the child asked
    /// for.
    #[test]
    fn a_fixed_extent_list_forces_the_item_extent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let sliver = SliverFixedExtentList::builder(
            |_app, _context, _index| Some(SizedBox::new().height(10.0).into_widget()),
            40.0,
        )
        .item_count(8)
        .into_widget();
        let harness = Harness::mount(&mut app, viewport_over(sliver, offset));
        harness.pump(&mut app);

        let list: RenderHandle<RenderSliverFixedExtentList> = first_sliver(&harness, &app);
        let children = children_of(list, &app);
        assert_eq!(children.len(), 5);
        for child in children {
            assert_eq!(child.size(&app).height(), 40.0);
        }
        assert_eq!(list.geometry(&app).scroll_extent, 320.0);
    }
}

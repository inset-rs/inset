//! Flutter counterpart: `rendering/sliver_multi_box_adaptor.dart`.
//!
//! Semantics wait; see `PORTING.md`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug, Display};
use std::rc::Rc;

use indexmap::IndexMap;
use inset_embedder::{Matrix4, Offset};
use inset_foundation::{App, Handle};
use inset_painting::{Axis, AxisDirection};

use crate::box_::{AnyRenderBox, BoxConstraints, BoxHitTestResult};
use crate::object::{
    AnyRenderObject, ContainerParentData, ContainerParentDataMixin, ContainerRenderObjectBase,
    ContainerRenderObjectMixin, ParentData, RenderHandle, zero_matrix,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::sliver::{
    RenderSliver, RenderSliverHelpers, SliverConstraints, SliverHitTestResult,
    SliverLogicalParentData, apply_growth_direction_to_axis_direction,
};

/// A delegate used by [`RenderSliverMultiBoxAdaptor`] to manage its children.
///
/// [`RenderSliverMultiBoxAdaptor`] objects reify their children lazily to avoid spending
/// resources on children that are not visible in the viewport. This delegate lets these objects
/// create and remove children as well as estimate the total scroll offset extent occupied by the
/// full child list.
///
/// The widget layer implements this; the render object holds it as an
/// [`Rc<dyn RenderSliverBoxChildManager>`].
pub trait RenderSliverBoxChildManager {
    /// Called during layout when a new child is needed.
    ///
    /// The child should be inserted into the child list in the appropriate position, after the
    /// `after` child (at the start of the list if `after` is `None`). Its index and scroll
    /// offsets will automatically be set appropriately.
    ///
    /// The `index` argument gives the index of the child to show. It is possible for negative
    /// indices to be requested. For example: if the user scrolls from child 0 to child 10, and
    /// then those children get much smaller, and then the user scrolls back up again, this
    /// method will eventually be asked to produce a child for index -1.
    ///
    /// If no child corresponds to `index`, then do nothing.
    ///
    /// Which child is indicated by index zero depends on the
    /// [`crate::GrowthDirection`] specified in the `constraints` of the
    /// [`RenderSliverMultiBoxAdaptor`].
    ///
    /// During a call to this method it is valid to remove other children from the
    /// [`RenderSliverMultiBoxAdaptor`] object if they were not created during this frame and have
    /// not yet been updated during this frame. It is not valid to add any other children to this
    /// render object.
    fn create_child(&self, app: &mut App, index: i32, after: Option<AnyRenderBox>);

    /// Remove the given child from the child list.
    ///
    /// Called by [`RenderSliverMultiBoxAdaptor::collect_garbage`], which itself is called from
    /// the adaptor's `perform_layout`.
    ///
    /// The index of the given child can be obtained using
    /// [`RenderSliverMultiBoxAdaptor::index_of`].
    fn remove_child(&self, app: &mut App, child: AnyRenderBox);

    /// Called to estimate the total scrollable extents of this object.
    ///
    /// Must return the total distance from the start of the child with the earliest possible
    /// index to the end of the child with the last possible index.
    fn estimate_max_scroll_offset(
        &self,
        app: &App,
        constraints: SliverConstraints,
        first_index: Option<i32>,
        last_index: Option<i32>,
        leading_scroll_offset: Option<f64>,
        trailing_scroll_offset: Option<f64>,
    ) -> f64;

    /// Called to obtain a precise measure of the total number of children.
    ///
    /// Must return the number that is one greater than the greatest `index` for which
    /// [`create_child`](Self::create_child) will actually create a child.
    ///
    /// This is used when `create_child` cannot add a child for a positive `index`, to determine
    /// the precise dimensions of the sliver. It will not be called if `create_child` is always
    /// able to create a child (e.g. for an infinite list).
    fn child_count(&self, app: &mut App) -> i32;

    /// The best available estimate of [`child_count`](Self::child_count), or `None` if no
    /// estimate is available.
    fn estimated_child_count(&self, app: &App) -> Option<i32> {
        let _ = app;
        None
    }

    /// Called during [`RenderSliverMultiBoxAdaptor::insert`] or
    /// [`RenderSliverMultiBoxAdaptor::move_child`].
    ///
    /// Implementors must ensure that the [`SliverMultiBoxAdaptorParentData::index`] field of the
    /// child's parent data accurately reflects the child's index in the child list after this
    /// function returns.
    fn did_adopt_child(&self, app: &mut App, child: AnyRenderBox);

    /// Called during layout to indicate whether this object provided insufficient children for
    /// the [`RenderSliverMultiBoxAdaptor`] to fill the
    /// [`SliverConstraints::remaining_paint_extent`].
    ///
    /// Typically called unconditionally at the start of layout with `false` and then later
    /// called with `true` when the adaptor fails to create a child required to fill the
    /// remaining paint extent.
    fn set_did_underflow(&self, app: &mut App, value: bool);

    /// Called at the beginning of layout to indicate that layout is about to occur.
    fn did_start_layout(&self, app: &mut App) {
        let _ = app;
    }

    /// Called at the end of layout to indicate that layout is now complete.
    fn did_finish_layout(&self, app: &mut App) {
        let _ = app;
    }

    /// In debug mode, asserts that this manager is not expecting any modifications to the
    /// [`RenderSliverMultiBoxAdaptor`]'s child list.
    ///
    /// This function always returns true.
    fn debug_assert_child_list_locked(&self, app: &App) -> bool {
        let _ = app;
        true
    }
}

/// Parent data structure used by [`RenderSliverWithKeepAliveMixin`].
///
/// Flutter's `KeepAliveParentDataMixin`: the half a parent data embeds and answers from
/// [`ParentData::provide`], which is how `KeepAlive` finds it.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeepAliveParentData {
    /// Whether to keep the child alive even when it is no longer visible.
    pub keep_alive: bool,
    kept_alive: bool,
}

impl KeepAliveParentData {
    /// Parent data that does not keep its child alive.
    pub const fn new() -> KeepAliveParentData {
        KeepAliveParentData {
            keep_alive: false,
            kept_alive: false,
        }
    }

    /// Whether the widget is currently being kept alive, i.e. has
    /// [`keep_alive`](Self::keep_alive) set to true and is offscreen.
    pub fn kept_alive(&self) -> bool {
        self.kept_alive
    }
}

/// This trait exists to dissociate `KeepAlive` from [`RenderSliverMultiBoxAdaptor`].
///
/// [`setup_parent_data`](Self::setup_parent_data) must be implemented to use a parent data type
/// that embeds a [`KeepAliveParentData`].
pub trait RenderSliverWithKeepAliveMixin: RenderSliver {
    /// Alerts the developer that the child's parent data needs to embed a
    /// [`KeepAliveParentData`].
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        let _ = self;
        debug_assert!(
            child
                .parent_data(app)
                .is_some_and(|parent_data| parent_data.part::<KeepAliveParentData>().is_some())
        );
    }
}

/// Parent data structure used by [`RenderSliverMultiBoxAdaptor`].
#[derive(Debug)]
pub struct SliverMultiBoxAdaptorParentData {
    sliver_logical: SliverLogicalParentData,
    container: ContainerParentData<AnyRenderBox>,
    keep_alive: KeepAliveParentData,
    /// The index of this child according to the [`RenderSliverBoxChildManager`].
    pub index: Option<i32>,
}

impl SliverMultiBoxAdaptorParentData {
    /// Creates parent data with no index and no layout offset.
    pub const fn new() -> SliverMultiBoxAdaptorParentData {
        SliverMultiBoxAdaptorParentData {
            sliver_logical: SliverLogicalParentData::new(),
            container: ContainerParentData::new(),
            keep_alive: KeepAliveParentData::new(),
            index: None,
        }
    }

    /// See [`SliverLogicalParentData::layout_offset`].
    pub fn layout_offset(&self) -> Option<f64> {
        self.sliver_logical.layout_offset
    }

    /// Sets [`layout_offset`](Self::layout_offset).
    pub fn set_layout_offset(&mut self, value: Option<f64>) {
        self.sliver_logical.layout_offset = value;
    }

    /// See [`KeepAliveParentData::keep_alive`].
    pub fn keep_alive(&self) -> bool {
        self.keep_alive.keep_alive
    }

    /// Sets [`keep_alive`](Self::keep_alive).
    pub fn set_keep_alive(&mut self, value: bool) {
        self.keep_alive.keep_alive = value;
    }

    /// See [`KeepAliveParentData::kept_alive`].
    pub fn kept_alive(&self) -> bool {
        self.keep_alive.kept_alive
    }
}

impl Default for SliverMultiBoxAdaptorParentData {
    fn default() -> SliverMultiBoxAdaptorParentData {
        SliverMultiBoxAdaptorParentData::new()
    }
}

impl ParentData for SliverMultiBoxAdaptorParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<SliverMultiBoxAdaptorParentData>() {
            return Some(self);
        }
        if id == TypeId::of::<KeepAliveParentData>() {
            return Some(&self.keep_alive);
        }
        self.sliver_logical.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<SliverMultiBoxAdaptorParentData>() {
            return Some(self);
        }
        if id == TypeId::of::<KeepAliveParentData>() {
            return Some(&mut self.keep_alive);
        }
        self.sliver_logical.provide_mut(id)
    }
}

impl ContainerParentDataMixin for SliverMultiBoxAdaptorParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container
    }
}

impl Display for SliverMultiBoxAdaptorParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "index={:?}; ", self.index)?;
        if self.keep_alive() {
            f.write_str("keepAlive; ")?;
        }
        Display::fmt(&self.sliver_logical, f)
    }
}

/// Flutter's `RenderSliverMultiBoxAdaptor` fields.
pub struct RenderSliverMultiBoxAdaptorData {
    child_manager: Rc<dyn RenderSliverBoxChildManager>,
    /// The nodes being kept alive despite not being visible.
    keep_alive_bucket: IndexMap<i32, AnyRenderBox>,
    debug_dangling_keep_alives: Vec<AnyRenderBox>,
    debug_child_integrity_enabled: bool,
}

impl RenderSliverMultiBoxAdaptorData {
    /// Creates the state of a sliver with multiple box children.
    pub fn new(
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
    ) -> RenderSliverMultiBoxAdaptorData {
        RenderSliverMultiBoxAdaptorData {
            child_manager,
            keep_alive_bucket: IndexMap::new(),
            debug_dangling_keep_alives: Vec::new(),
            debug_child_integrity_enabled: true,
        }
    }
}

/// Implements [`RenderSliverMultiBoxAdaptor`] field accessors for an `adaptor` field.
#[macro_export]
macro_rules! render_sliver_multi_box_adaptor_accessors {
    () => {
        fn adaptor_data(
            self: $crate::RenderHandle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::RenderSliverMultiBoxAdaptorData {
            &self.get(app).adaptor
        }
        fn adaptor_data_mut(
            self: $crate::RenderHandle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::RenderSliverMultiBoxAdaptorData {
            &mut self.get_mut(app).adaptor
        }
    };
}

/// A sliver with multiple box children.
///
/// [`RenderSliverMultiBoxAdaptor`] is a base class for slivers that have multiple box children.
/// The children are managed by a [`RenderSliverBoxChildManager`], which lets implementors create
/// children lazily during layout. Typically implementors will create only those children that are
/// actually needed to fill the [`SliverConstraints::remaining_paint_extent`].
///
/// The contract for adding and removing children from this render object is more strict than for
/// normal render objects:
///
/// * Children can be removed except during a layout pass if they have already been laid out
///   during that layout pass.
/// * Children cannot be added except during a call to
///   [`child_manager`](Self::child_manager), and then only if there is no child corresponding to
///   that index (or the child corresponding to that index was first removed).
///
/// See also:
///
///  * [`crate::RenderSliverToBoxAdapter`], which has a single box child.
///  * [`crate::RenderSliverList`], which places its children in a linear array.
///  * [`crate::RenderSliverFixedExtentList`], which places its children in a linear array with a
///    fixed extent in the main axis.
pub trait RenderSliverMultiBoxAdaptor:
    RenderSliver
    + RenderSliverHelpers
    + RenderSliverWithKeepAliveMixin
    + ContainerRenderObjectMixin<
        ChildType = AnyRenderBox,
        ParentDataType = SliverMultiBoxAdaptorParentData,
    >
{
    /// Mixin field access.
    fn adaptor_data(self: RenderHandle<Self>, app: &App) -> &RenderSliverMultiBoxAdaptorData;

    /// See [`adaptor_data`](Self::adaptor_data).
    fn adaptor_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverMultiBoxAdaptorData;

    /// The delegate that manages the children of this object.
    ///
    /// Rather than having a concrete list of children, a [`RenderSliverMultiBoxAdaptor`] uses a
    /// [`RenderSliverBoxChildManager`] to create children during layout in order to fill the
    /// [`SliverConstraints::remaining_paint_extent`].
    fn child_manager(self: RenderHandle<Self>, app: &App) -> Rc<dyn RenderSliverBoxChildManager> {
        Rc::clone(&self.adaptor_data(app).child_manager)
    }

    /// The body of Flutter's `setupParentData` override.
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<SliverMultiBoxAdaptorParentData>(app) {
            child.set_parent_data(app, SliverMultiBoxAdaptorParentData::new());
        }
    }

    /// Whether the integrity check is enabled.
    ///
    /// The integrity check consists of:
    ///
    /// 1. Verify that the children index in the child list is in ascending order.
    /// 2. Verify that there is no dangling keep-alive child as the result of
    ///    [`move_child`](Self::move_child).
    fn debug_child_integrity_enabled(self: RenderHandle<Self>, app: &App) -> bool {
        self.adaptor_data(app).debug_child_integrity_enabled
    }

    /// Sets [`debug_child_integrity_enabled`](Self::debug_child_integrity_enabled), immediately
    /// performing an integrity check.
    fn set_debug_child_integrity_enabled(self: RenderHandle<Self>, app: &mut App, enabled: bool) {
        if !cfg!(debug_assertions) {
            return;
        }
        self.adaptor_data_mut(app).debug_child_integrity_enabled = enabled;
        debug_assert!(self.debug_verify_child_order(app));
        debug_assert!(
            !enabled || self.adaptor_data(app).debug_dangling_keep_alives.is_empty(),
            "a keep-alive child was left dangling by move_child"
        );
    }

    /// Flutter's `_debugAssertChildListLocked`.
    fn debug_assert_child_list_locked(self: RenderHandle<Self>, app: &App) -> bool {
        self.child_manager(app).debug_assert_child_list_locked(app)
    }

    /// Verify that the child list index is in strictly increasing order.
    ///
    /// This has no effect in release builds.
    fn debug_verify_child_order(self: RenderHandle<Self>, app: &App) -> bool {
        if self.debug_child_integrity_enabled(app) {
            let mut child = self.first_child(app);
            while let Some(current) = child {
                let index = self.index_of(app, current);
                child = self.child_after(app, current);
                debug_assert!(child.is_none_or(|next| self.index_of(app, next) > index));
            }
        }
        true
    }

    /// The body of Flutter's `insert` override.
    fn insert(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        after: Option<AnyRenderBox>,
    ) {
        debug_assert!(
            !self
                .adaptor_data(app)
                .keep_alive_bucket
                .values()
                .any(|kept| *kept == child)
        );
        ContainerRenderObjectBase::insert(self, app, child, after);
        self.did_adopt_child(app, child);
        debug_assert!(self.first_child(app).is_some());
        debug_assert!(self.debug_verify_child_order(app));
    }

    /// The body of Flutter's `adoptChild` override, run after a child joins the child list.
    ///
    /// Flutter overrides `RenderObject.adoptChild`; here the tree operation is not virtual, so
    /// the two callers ([`insert`](Self::insert) and Dart's `_destroyOrCacheChild`, which takes
    /// `super.adoptChild`) decide whether it runs.
    fn did_adopt_child(self: RenderHandle<Self>, app: &mut App, child: AnyRenderBox) {
        let kept_alive = child
            .as_object()
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
            .kept_alive();
        if !kept_alive {
            self.child_manager(app).did_adopt_child(app, child);
        }
    }

    /// The body of Flutter's `move` override.
    ///
    /// There are two scenarios:
    ///
    /// 1. The child is not kept alive. The child is in the child list maintained by
    ///    [`ContainerRenderObjectMixin`]. We can call the inherited body and update the parent
    ///    data with the new slot.
    ///
    /// 2. The child is kept alive. In this case, the child is no longer in the child list but
    ///    might be stored in the keep-alive bucket. We need to update the location of the child
    ///    in the bucket.
    fn move_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        after: Option<AnyRenderBox>,
    ) {
        let child_parent_data = child
            .as_object()
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app);
        let (kept_alive, index) = (child_parent_data.kept_alive(), child_parent_data.index);
        if !kept_alive {
            ContainerRenderObjectBase::move_child(self, app, child, after);
            // updates the slot in the parent data
            self.child_manager(app).did_adopt_child(app, child);
            // Its slot may change even if the inherited body does not change the position. In
            // this case, we still want to mark as needs layout.
            self.mark_needs_layout(app);
            return;
        }
        // If the child in the bucket is not the current child, that means someone has already
        // moved and replaced the current child, and we cannot remove this child.
        if let Some(index) = index
            && self.adaptor_data(app).keep_alive_bucket.get(&index) == Some(&child)
        {
            self.adaptor_data_mut(app)
                .keep_alive_bucket
                .shift_remove(&index);
        }
        if cfg!(debug_assertions) {
            self.adaptor_data_mut(app)
                .debug_dangling_keep_alives
                .retain(|dangling| *dangling != child);
        }
        // Update the slot and reinsert back to the keep-alive bucket in the new slot.
        self.child_manager(app).did_adopt_child(app, child);
        // If there is an existing child in the new slot, that means that child will be moved to
        // another index. In other cases, the existing child should have been removed by
        // update_child. Thus, it is ok to overwrite it.
        let new_index = child
            .as_object()
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
            .index
            .expect("did_adopt_child sets the index");
        if cfg!(debug_assertions)
            && let Some(existing) = self
                .adaptor_data(app)
                .keep_alive_bucket
                .get(&new_index)
                .copied()
        {
            self.adaptor_data_mut(app)
                .debug_dangling_keep_alives
                .push(existing);
        }
        self.adaptor_data_mut(app)
            .keep_alive_bucket
            .insert(new_index, child);
    }

    /// The body of Flutter's `remove` override.
    fn remove(self: RenderHandle<Self>, app: &mut App, child: AnyRenderBox) {
        let child_parent_data = child
            .as_object()
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app);
        let (kept_alive, index) = (child_parent_data.kept_alive(), child_parent_data.index);
        if !kept_alive {
            ContainerRenderObjectBase::remove(self, app, child);
            return;
        }
        let index = index.expect("a kept-alive child has an index");
        debug_assert_eq!(
            self.adaptor_data(app).keep_alive_bucket.get(&index),
            Some(&child)
        );
        if cfg!(debug_assertions) {
            self.adaptor_data_mut(app)
                .debug_dangling_keep_alives
                .retain(|dangling| *dangling != child);
        }
        self.adaptor_data_mut(app)
            .keep_alive_bucket
            .shift_remove(&index);
        self.drop_child(app, child.as_object());
    }

    /// The body of Flutter's `removeAll` override.
    fn remove_all(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectBase::remove_all(self, app);
        for child in self.keep_alive_children(app) {
            self.drop_child(app, child.as_object());
        }
        self.adaptor_data_mut(app).keep_alive_bucket.clear();
    }

    /// The children currently held in the keep-alive bucket, in insertion order.
    fn keep_alive_children(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        self.adaptor_data(app)
            .keep_alive_bucket
            .values()
            .copied()
            .collect()
    }

    /// Flutter's `_createOrObtainChild`.
    fn create_or_obtain_child(
        self: RenderHandle<Self>,
        app: &mut App,
        index: i32,
        after: Option<AnyRenderBox>,
    ) {
        self.as_object().invoke_layout_callback(app, |app| {
            let cached = self
                .adaptor_data(app)
                .keep_alive_bucket
                .get(&index)
                .copied();
            let Some(child) = cached else {
                self.child_manager(app).create_child(app, index, after);
                return;
            };
            self.adaptor_data_mut(app)
                .keep_alive_bucket
                .shift_remove(&index);
            let child_parent_data = child
                .as_object()
                .parent_data_of::<SliverMultiBoxAdaptorParentData>(app);
            debug_assert!(child_parent_data.kept_alive());
            // Dart re-assigns the parent data it saved before `dropChild` nulled it; the fields
            // are plain data, so the saved values are rebuilt instead.
            let (layout_offset, keep_alive, index_of_child) = (
                child_parent_data.layout_offset(),
                child_parent_data.keep_alive(),
                child_parent_data.index,
            );
            self.drop_child(app, child.as_object());
            let mut restored = SliverMultiBoxAdaptorParentData::new();
            restored.set_layout_offset(layout_offset);
            restored.set_keep_alive(keep_alive);
            restored.index = index_of_child;
            restored.keep_alive.kept_alive = true;
            child.as_object().set_parent_data(app, restored);
            RenderSliverMultiBoxAdaptor::insert(self, app, child, after);
            child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .keep_alive
                .kept_alive = false;
        });
    }

    /// Flutter's `_destroyOrCacheChild`.
    fn destroy_or_cache_child(self: RenderHandle<Self>, app: &mut App, child: AnyRenderBox) {
        let child_parent_data = child
            .as_object()
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app);
        if !child_parent_data.keep_alive() {
            debug_assert_eq!(child.as_object().parent(app), Some(self.as_object()));
            self.child_manager(app).remove_child(app, child);
            debug_assert!(child.as_object().parent(app).is_none());
            return;
        }
        debug_assert!(!child_parent_data.kept_alive());
        let (layout_offset, index) = (child_parent_data.layout_offset(), child_parent_data.index);
        let index = index.expect("a cached child has an index");
        RenderSliverMultiBoxAdaptor::remove(self, app, child);
        self.adaptor_data_mut(app)
            .keep_alive_bucket
            .insert(index, child);
        let mut restored = SliverMultiBoxAdaptorParentData::new();
        restored.set_layout_offset(layout_offset);
        restored.set_keep_alive(true);
        restored.index = Some(index);
        restored.keep_alive.kept_alive = true;
        child.as_object().set_parent_data(app, restored);
        // Dart's `super.adoptChild`: the child manager is not told, because the child keeps its
        // slot.
        self.as_object().adopt_child(app, child.as_object());
    }

    /// The body of Flutter's `attach` override: attaches the kept-alive children too.
    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
        for child in self.keep_alive_children(app) {
            child.as_object().attach(app, owner);
        }
    }

    /// The body of Flutter's `detach` override: detaches the kept-alive children too.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app);
        for child in self.keep_alive_children(app) {
            child.as_object().detach(app);
        }
    }

    /// The body of Flutter's `redepthChildren` override.
    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app);
        for child in self.keep_alive_children(app) {
            self.as_object().redepth_child(app, child.as_object());
        }
    }

    /// The body of Flutter's `visitChildren` override: visits the kept-alive children too.
    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor);
        for child in self.keep_alive_children(app) {
            visitor(child.as_object());
        }
    }

    /// Called during layout to create and add the child with the given index and scroll offset.
    ///
    /// Calls [`RenderSliverBoxChildManager::create_child`] to actually create and add the child
    /// if necessary. The child may instead be obtained from a cache; see
    /// [`KeepAliveParentData::keep_alive`].
    ///
    /// Returns `false` if there was no cached child and `create_child` did not add any child,
    /// otherwise returns `true`.
    ///
    /// Does not lay out the new child.
    fn add_initial_child(
        self: RenderHandle<Self>,
        app: &mut App,
        index: i32,
        layout_offset: f64,
    ) -> bool {
        debug_assert!(self.debug_assert_child_list_locked(app));
        debug_assert!(self.first_child(app).is_none());
        self.create_or_obtain_child(app, index, None);
        if let Some(first_child) = self.first_child(app) {
            debug_assert_eq!(Some(first_child), self.last_child(app));
            debug_assert_eq!(self.index_of(app, first_child), index);
            first_child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .set_layout_offset(Some(layout_offset));
            return true;
        }
        self.child_manager(app).set_did_underflow(app, true);
        false
    }

    /// Called during layout to create, add, and lay out the child before
    /// [`ContainerRenderObjectMixin::first_child`].
    ///
    /// Returns the new child or `None` if no child was obtained.
    ///
    /// The child that was previously the first child, as well as any subsequent children, may be
    /// removed by this call if they have not yet been laid out during this layout pass.
    fn insert_and_layout_leading_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child_constraints: BoxConstraints,
        parent_uses_size: bool,
    ) -> Option<AnyRenderBox> {
        debug_assert!(self.debug_assert_child_list_locked(app));
        let first_child = self.first_child(app).expect("there is a leading child");
        let index = self.index_of(app, first_child) - 1;
        self.create_or_obtain_child(app, index, None);
        let first_child = self.first_child(app).expect("the child list is not empty");
        if self.index_of(app, first_child) == index {
            first_child.layout(app, child_constraints, parent_uses_size);
            return Some(first_child);
        }
        self.child_manager(app).set_did_underflow(app, true);
        None
    }

    /// Called during layout to create, add, and lay out the child after the given child.
    ///
    /// Returns the new child. It is the responsibility of the caller to configure the child's
    /// scroll offset.
    ///
    /// Children after the `after` child may be removed in the process. Only the new child may be
    /// added.
    fn insert_and_layout_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child_constraints: BoxConstraints,
        after: Option<AnyRenderBox>,
        parent_uses_size: bool,
    ) -> Option<AnyRenderBox> {
        debug_assert!(self.debug_assert_child_list_locked(app));
        let after = after.expect("insert_and_layout_child needs a preceding child");
        let index = self.index_of(app, after) + 1;
        self.create_or_obtain_child(app, index, Some(after));
        let child = self.child_after(app, after);
        if let Some(child) = child
            && self.index_of(app, child) == index
        {
            child.layout(app, child_constraints, parent_uses_size);
            return Some(child);
        }
        self.child_manager(app).set_did_underflow(app, true);
        None
    }

    /// Returns the number of children preceding the `first_index` that need to be garbage
    /// collected.
    fn calculate_leading_garbage(self: RenderHandle<Self>, app: &App, first_index: i32) -> usize {
        let mut walker = self.first_child(app);
        let mut leading_garbage = 0;
        while let Some(current) = walker {
            if self.index_of(app, current) >= first_index {
                break;
            }
            leading_garbage += 1;
            walker = self.child_after(app, current);
        }
        leading_garbage
    }

    /// Returns the number of children following the `last_index` that need to be garbage
    /// collected.
    fn calculate_trailing_garbage(self: RenderHandle<Self>, app: &App, last_index: i32) -> usize {
        let mut walker = self.last_child(app);
        let mut trailing_garbage = 0;
        while let Some(current) = walker {
            if self.index_of(app, current) <= last_index {
                break;
            }
            trailing_garbage += 1;
            walker = self.child_before(app, current);
        }
        trailing_garbage
    }

    /// Called after layout with the number of children that can be garbage collected at the head
    /// and tail of the child list.
    ///
    /// Children whose [`KeepAliveParentData::keep_alive`] property is set to true will be
    /// removed to a cache instead of being dropped.
    ///
    /// This method also collects any children that were previously kept alive but are now no
    /// longer necessary. As such, it should be called every time `perform_layout` is run, even if
    /// the arguments are both zero.
    fn collect_garbage(
        self: RenderHandle<Self>,
        app: &mut App,
        leading_garbage: usize,
        trailing_garbage: usize,
    ) {
        debug_assert!(self.debug_assert_child_list_locked(app));
        debug_assert!(self.child_count(app) >= leading_garbage + trailing_garbage);
        self.as_object().invoke_layout_callback(app, |app| {
            for _ in 0..leading_garbage {
                let first_child = self.first_child(app).expect("counted above");
                self.destroy_or_cache_child(app, first_child);
            }
            for _ in 0..trailing_garbage {
                let last_child = self.last_child(app).expect("counted above");
                self.destroy_or_cache_child(app, last_child);
            }
            // Ask the child manager to remove the children that are no longer being kept alive.
            // (This should cause the keep-alive bucket to change, so we have to prepare our list
            // ahead of time.)
            let expired: Vec<AnyRenderBox> = self
                .keep_alive_children(app)
                .into_iter()
                .filter(|child| {
                    !child
                        .as_object()
                        .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                        .keep_alive()
                })
                .collect();
            let child_manager = self.child_manager(app);
            for child in expired {
                child_manager.remove_child(app, child);
            }
            debug_assert!(self.keep_alive_children(app).iter().all(|child| {
                child
                    .as_object()
                    .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                    .keep_alive()
            }));
        });
    }

    /// Returns the index of the given child, as given by the
    /// [`SliverMultiBoxAdaptorParentData::index`] field of the child's parent data.
    fn index_of(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> i32 {
        child
            .as_object()
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
            .index
            .expect("a reified child has an index")
    }

    /// Returns the dimension of the given child in the main axis, as given by the child's size.
    ///
    /// This is only valid after layout.
    fn paint_extent_of(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> f64 {
        debug_assert!(child.has_size(app));
        match self.constraints(app).axis() {
            Axis::Horizontal => child.size(app).width(),
            Axis::Vertical => child.size(app).height(),
        }
    }

    /// The body of Flutter's `hitTestChildren` override.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        let mut child = self.last_child(app);
        while let Some(current) = child {
            let previous = self.child_before(app, current);
            if self.hit_test_box_child(
                app,
                &mut BoxHitTestResult::wrap(result),
                current,
                main_axis_position,
                cross_axis_position,
            ) {
                return true;
            }
            child = previous;
        }
        false
    }

    /// The body of Flutter's `childMainAxisPosition` override.
    fn child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        RenderSliverMultiBoxAdaptor::child_scroll_offset(self, app, child)
            .expect("a laid-out child has a scroll offset")
            - self.constraints(app).scroll_offset
    }

    /// The body of Flutter's `childScrollOffset` override.
    fn child_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> Option<f64> {
        debug_assert_eq!(child.parent(app), Some(self.as_object()));
        child
            .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
            .layout_offset()
    }

    /// Whether the given child is painted, i.e. it is reified and not kept alive off-screen.
    ///
    /// Flutter's `paintsChild` override; `RenderObject.paintsChild` is not a virtual here (see
    /// `PORTING.md`).
    fn paints_child(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> bool {
        let index = child
            .as_object()
            .parent_data(app)
            .and_then(|parent_data| {
                (parent_data as &dyn std::any::Any)
                    .downcast_ref::<SliverMultiBoxAdaptorParentData>()
            })
            .and_then(|parent_data| parent_data.index);
        match index {
            None => false,
            Some(index) => !self
                .adaptor_data(app)
                .keep_alive_bucket
                .contains_key(&index),
        }
    }

    /// The body of Flutter's `applyPaintTransform` override.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let child = child.as_box().expect("the children are boxes");
        if self.paints_child(app, child) {
            self.apply_paint_transform_for_box_child(app, child, transform);
        } else {
            // This can happen if some child asks for the global transform even though they are
            // not getting painted. In that case, the transform is set to zero since
            // apply_paint_transform_for_box_child would end up throwing due to the child not
            // being configured correctly for applying a transform. There's no assert here because
            // asking for the paint transform is a valid thing to do even if a child would not be
            // painted, but there is no meaningful non-zero matrix to use in this case.
            *transform = zero_matrix();
        }
    }

    /// The body of Flutter's `paint` override.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.first_child(app).is_none() {
            return;
        }
        // offset is to the top-left corner, regardless of our axis direction.
        // origin_offset gives us the delta from the real origin to the origin in the axis
        // direction.
        let constraints = self.constraints(app);
        let paint_extent = self.geometry(app).paint_extent;
        let (main_axis_unit, cross_axis_unit, origin_offset, add_extent) =
            match apply_growth_direction_to_axis_direction(
                constraints.axis_direction,
                constraints.growth_direction,
            ) {
                AxisDirection::Up => (
                    Offset::new(0.0, -1.0),
                    Offset::new(1.0, 0.0),
                    offset + Offset::new(0.0, paint_extent),
                    true,
                ),
                AxisDirection::Right => {
                    (Offset::new(1.0, 0.0), Offset::new(0.0, 1.0), offset, false)
                }
                AxisDirection::Down => {
                    (Offset::new(0.0, 1.0), Offset::new(1.0, 0.0), offset, false)
                }
                AxisDirection::Left => (
                    Offset::new(-1.0, 0.0),
                    Offset::new(0.0, 1.0),
                    offset + Offset::new(paint_extent, 0.0),
                    true,
                ),
            };
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let main_axis_delta = RenderSliverMultiBoxAdaptor::child_main_axis_position(
                self,
                app,
                current.as_object(),
            );
            let cross_axis_delta = self.child_cross_axis_position(app, current.as_object());
            let mut child_offset = Offset::new(
                origin_offset.dx()
                    + main_axis_unit.dx() * main_axis_delta
                    + cross_axis_unit.dx() * cross_axis_delta,
                origin_offset.dy()
                    + main_axis_unit.dy() * main_axis_delta
                    + cross_axis_unit.dy() * cross_axis_delta,
            );
            let child_paint_extent = self.paint_extent_of(app, current);
            if add_extent {
                child_offset = child_offset + main_axis_unit * child_paint_extent;
            }

            // If the child's visible interval (main_axis_delta, main_axis_delta +
            // paint_extent_of(child)) does not intersect the paint extent interval
            // (0, constraints.remaining_paint_extent), it's hidden.
            if main_axis_delta < constraints.remaining_paint_extent
                && main_axis_delta + child_paint_extent > 0.0
            {
                context.paint_child(app, current.as_object(), child_offset);
            }

            child = self.child_after(app, current);
        }
    }

    /// Asserts that the reified child list is not empty and has a contiguous sequence of indices.
    ///
    /// Always returns true.
    fn debug_assert_child_list_is_non_empty_and_contiguous(
        self: RenderHandle<Self>,
        app: &App,
    ) -> bool {
        if cfg!(debug_assertions) {
            let first_child = self.first_child(app).expect("the child list is not empty");
            let mut index = self.index_of(app, first_child);
            let mut child = self.child_after(app, first_child);
            while let Some(current) = child {
                index += 1;
                debug_assert_eq!(self.index_of(app, current), index);
                child = self.child_after(app, current);
            }
        }
        true
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::{Cell, RefCell};

    use inset_embedder::Size;

    use super::*;
    use crate::box_::{RenderBox, RenderBoxData};
    use crate::object::{RenderObject, RenderObjectData};

    /// A box that takes the largest size the constraints allow, and accepts hits.
    pub(crate) struct TestChild {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        preferred_extent: f64,
    }

    impl RenderObject for TestChild {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let extent = self.get(app).preferred_extent;
            let size = self.constraints(app).constrain(Size::new(extent, extent));
            self.set_size(app, size);
        }
    }

    impl RenderBox for TestChild {
        crate::render_box_accessors!();

        fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
            let _ = self;
            true
        }
    }

    /// A box that fills the constraints it is given, up to `extent` in each dimension, and
    /// accepts hits.
    pub(crate) fn hit_testable_box(app: &mut App, extent: f64) -> AnyRenderBox {
        RenderHandle::new_box(
            app,
            TestChild {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                preferred_extent: extent,
            },
        )
        .as_box()
    }

    /// A [`RenderSliverBoxChildManager`] over a fixed list of child extents, as
    /// `SliverMultiBoxAdaptorElement` is over a delegate.
    pub(crate) struct TestChildManager<T: RenderSliverMultiBoxAdaptor> {
        render_object: Cell<Option<RenderHandle<T>>>,
        extents: Vec<f64>,
        currently_updating_child_index: Cell<Option<i32>>,
        /// The indices `create_child` was asked for, in order.
        pub(crate) created: RefCell<Vec<i32>>,
        /// The last value passed to `set_did_underflow`.
        pub(crate) did_underflow: Cell<bool>,
    }

    impl<T: RenderSliverMultiBoxAdaptor> TestChildManager<T> {
        pub(crate) fn new(extents: Vec<f64>) -> Rc<TestChildManager<T>> {
            Rc::new(TestChildManager {
                render_object: Cell::new(None),
                extents,
                currently_updating_child_index: Cell::new(None),
                created: RefCell::new(Vec::new()),
                did_underflow: Cell::new(false),
            })
        }

        pub(crate) fn attach(&self, render_object: RenderHandle<T>) {
            self.render_object.set(Some(render_object));
        }

        fn adaptor(&self) -> RenderHandle<T> {
            self.render_object.get().expect("attached before layout")
        }

        fn total_extent(&self) -> f64 {
            self.extents.iter().sum()
        }
    }

    impl<T: RenderSliverMultiBoxAdaptor> RenderSliverBoxChildManager for TestChildManager<T> {
        fn create_child(&self, app: &mut App, index: i32, after: Option<AnyRenderBox>) {
            self.created.borrow_mut().push(index);
            let Ok(slot) = usize::try_from(index) else {
                return;
            };
            let Some(extent) = self.extents.get(slot).copied() else {
                return;
            };
            let child = RenderHandle::new_box(
                app,
                TestChild {
                    render_object: RenderObjectData::new(),
                    render_box: RenderBoxData::new(),
                    preferred_extent: extent,
                },
            )
            .as_box();
            self.currently_updating_child_index.set(Some(index));
            RenderSliverMultiBoxAdaptor::insert(self.adaptor(), app, child, after);
            self.currently_updating_child_index.set(None);
        }

        fn remove_child(&self, app: &mut App, child: AnyRenderBox) {
            // Dart's element unmounts the child and disposes its render object later; the test
            // leaves the arena slot alone so that assertions can still name the child.
            RenderSliverMultiBoxAdaptor::remove(self.adaptor(), app, child);
        }

        fn estimate_max_scroll_offset(
            &self,
            _app: &App,
            _constraints: SliverConstraints,
            _first_index: Option<i32>,
            _last_index: Option<i32>,
            _leading_scroll_offset: Option<f64>,
            _trailing_scroll_offset: Option<f64>,
        ) -> f64 {
            self.total_extent()
        }

        fn child_count(&self, _app: &mut App) -> i32 {
            self.extents.len() as i32
        }

        fn estimated_child_count(&self, _app: &App) -> Option<i32> {
            Some(self.extents.len() as i32)
        }

        fn did_adopt_child(&self, app: &mut App, child: AnyRenderBox) {
            child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .index = self.currently_updating_child_index.get();
        }

        fn set_did_underflow(&self, _app: &mut App, value: bool) {
            self.did_underflow.set(value);
        }
    }
}

#[cfg(test)]
pub(crate) mod viewport_test_support {
    use std::rc::Rc;

    use inset_embedder::{Picture, Size, View, ViewId, ViewMetrics, ViewRef};
    use inset_foundation::Handle;

    use super::*;
    use crate::box_::RenderBox;
    use crate::pipeline_owner::PipelineOwner;
    use crate::sliver::AnyRenderSliver;
    use crate::view::{RenderView, ViewConfiguration};
    use crate::viewport::RenderViewport;
    use crate::viewport_offset::{FixedViewportOffset, ViewportOffset};

    struct TestView;

    impl View for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            ViewMetrics::default()
        }

        fn present(&self, _picture: &Picture) {}
    }

    /// The scrolling machinery a lazy sliver needs: a viewport inside a [`RenderView`] rooted in
    /// a [`PipelineOwner`], laid out once.
    pub(crate) struct ScrollHarness {
        pub(crate) owner: Handle<PipelineOwner>,
        pub(crate) viewport: RenderHandle<RenderViewport>,
        pub(crate) offset: Handle<FixedViewportOffset>,
    }

    impl ScrollHarness {
        /// Lays `sliver` out in a downward-scrolling viewport of `size`.
        pub(crate) fn new(app: &mut App, sliver: AnyRenderSliver, size: Size) -> ScrollHarness {
            let offset = FixedViewportOffset::zero(app);
            let viewport = RenderViewport::new(
                app,
                AxisDirection::Right,
                offset.as_viewport_offset(),
                Some(vec![sliver]),
                None,
            );
            let constraints = BoxConstraints::tight(size);
            let configuration = ViewConfiguration::new(constraints, constraints, 1.0);
            let view = RenderView::new(
                app,
                Some(viewport.as_box()),
                Some(configuration),
                Rc::new(TestView) as ViewRef,
            );
            let owner = PipelineOwner::new(app, None);
            owner.set_root_node(app, Some(view.as_object()));
            view.prepare_initial_frame(app);
            owner.flush_layout(app);
            ScrollHarness {
                owner,
                viewport,
                offset,
            }
        }

        /// Scrolls to `pixels` and lays out again.
        pub(crate) fn scroll_to(&self, app: &mut App, pixels: f64) {
            let current = self.offset.pixels(app);
            self.offset.correct_by(app, pixels - current);
            self.viewport.mark_needs_layout(app);
            self.owner.flush_layout(app);
        }
    }
}

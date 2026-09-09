//! Flutter counterpart: `rendering/custom_layout.dart` (`MultiChildLayoutParentData`,
//! `MultiChildLayoutDelegate`, `RenderCustomMultiChildLayoutBox`).
//!
//! `SingleChildLayoutDelegate` and `RenderCustomSingleChildLayoutBox` live in
//! `shifted_box.dart`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Offset, Size};
use reveal_foundation::{App, Handle, Key, Listenable, Listener};

use crate::box_::{
    AnyRenderBox, BoxConstraints, BoxHitTestResult, BoxParentData, ContainerBoxParentData,
    RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
};
use crate::object::{
    AnyRenderObject, Constraints, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject,
    RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

/// An object representing the identity of a child of a [`RenderCustomMultiChildLayoutBox`].
///
/// Dart's `Object` id, which `LayoutId` also wraps in a `ValueKey` to key the widget.
pub type ChildLayoutId = Rc<dyn Key>;

/// [`ParentData`] used by [`RenderCustomMultiChildLayoutBox`].
#[derive(Debug)]
pub struct MultiChildLayoutParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,

    /// An object representing the identity of this child.
    pub id: Option<ChildLayoutId>,
}

impl MultiChildLayoutParentData {
    /// Creates parent data for a child that has no id yet.
    pub const fn new() -> MultiChildLayoutParentData {
        MultiChildLayoutParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            id: None,
        }
    }
}

impl Default for MultiChildLayoutParentData {
    fn default() -> MultiChildLayoutParentData {
        MultiChildLayoutParentData::new()
    }
}

impl ParentData for MultiChildLayoutParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<MultiChildLayoutParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<MultiChildLayoutParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for MultiChildLayoutParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for MultiChildLayoutParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for MultiChildLayoutParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; id={:?}", self.box_parent_data, self.id)
    }
}

/// The children a [`MultiChildLayoutDelegate`] lays out, keyed by their
/// [`MultiChildLayoutParentData::id`].
///
/// Dart's `MultiChildLayoutDelegate._idToChild` and, in debug builds, its
/// `_debugChildrenNeedingLayout`. [`RenderCustomMultiChildLayoutBox`] builds one from its child
/// list and hands it to [`MultiChildLayoutDelegate::perform_layout`].
pub struct MultiChildLayoutChildren {
    id_to_child: HashMap<ChildLayoutId, AnyRenderBox>,
    #[cfg(debug_assertions)]
    children_needing_layout: std::collections::HashSet<AnyRenderBox>,
}

impl MultiChildLayoutChildren {
    /// True if a non-`None` child was provided for the specified id.
    ///
    /// Call this from [`MultiChildLayoutDelegate::perform_layout`] to determine which children
    /// are available, if the child list might vary.
    ///
    /// This cannot be called from [`MultiChildLayoutDelegate::get_size`] as the size is not
    /// allowed to depend on the children.
    pub fn has_child(&self, child_id: &ChildLayoutId) -> bool {
        self.id_to_child.contains_key(child_id)
    }

    /// Ask the child to update its layout within the limits specified by the constraints
    /// parameter. The child's size is returned.
    ///
    /// Call this from [`MultiChildLayoutDelegate::perform_layout`] to lay out each child. Every
    /// child must be laid out using this function exactly once each time
    /// [`MultiChildLayoutDelegate::perform_layout`] is called.
    ///
    /// # Panics
    ///
    /// If there is no child with `child_id`, or, in a debug build, if that child was already
    /// laid out during this call.
    pub fn layout_child(
        &mut self,
        app: &mut App,
        child_id: &ChildLayoutId,
        constraints: BoxConstraints,
    ) -> Size {
        let child = *self.id_to_child.get(child_id).unwrap_or_else(|| {
            panic!(
                "The custom multichild layout delegate tried to lay out a non-existent child. \
                 There is no child with the id {child_id:?}."
            )
        });
        #[cfg(debug_assertions)]
        assert!(
            self.children_needing_layout.remove(&child),
            "The custom multichild layout delegate tried to lay out the child with id \
             {child_id:?} more than once. Each child must be laid out exactly once."
        );
        debug_assert!(
            constraints.debug_assert_is_valid(true),
            "The custom multichild layout delegate provided invalid box constraints for the \
             child with id {child_id:?}. The minimum width and height must be greater than or \
             equal to zero. The maximum width must be greater than or equal to the minimum \
             width. The maximum height must be greater than or equal to the minimum height."
        );
        child.layout(app, constraints, true);
        child.size(app)
    }

    /// Specify the child's origin relative to this origin.
    ///
    /// Call this from [`MultiChildLayoutDelegate::perform_layout`] to position each child. If
    /// you do not call this for a child, its position will remain unchanged. Children initially
    /// have their position set to (0,0), i.e. the top left of the
    /// [`RenderCustomMultiChildLayoutBox`].
    ///
    /// # Panics
    ///
    /// If there is no child with `child_id`.
    pub fn position_child(&self, app: &mut App, child_id: &ChildLayoutId, offset: Offset) {
        let child = *self.id_to_child.get(child_id).unwrap_or_else(|| {
            panic!(
                "The custom multichild layout delegate tried to position out a non-existent \
                 child: there is no child with the id {child_id:?}."
            )
        });
        child
            .parent_data_of_mut::<MultiChildLayoutParentData>(app)
            .set_offset(offset);
    }
}

/// A delegate that controls the layout of multiple children.
///
/// Used with `CustomMultiChildLayout` (in the widgets library) and
/// [`RenderCustomMultiChildLayoutBox`] (in the rendering library).
///
/// Delegates must be idempotent. Specifically, if two delegates are equal, then they must
/// produce the same layout. To change the layout, replace the delegate with a different
/// instance whose [`should_relayout`](Self::should_relayout) returns true when given the
/// previous instance.
///
/// Override [`get_size`](Self::get_size) to control the overall size of the layout. The size of
/// the layout cannot depend on layout properties of the children. This was a design decision to
/// simplify the delegate implementations: this way, the delegate implementations do not have to
/// also handle various intrinsic sizing functions if the parent's size depended on the children.
///
/// Override [`perform_layout`](Self::perform_layout) to size and position the children. An
/// implementation of [`perform_layout`](Self::perform_layout) must call
/// [`MultiChildLayoutChildren::layout_child`] exactly once for each child, but it may lay out
/// children in an arbitrary order.
///
/// Override [`should_relayout`](Self::should_relayout) to determine when the layout of the
/// children needs to be recomputed when the delegate changes.
///
/// The most efficient way to trigger a relayout is to answer a [`relayout`](Self::relayout)
/// listenable. The custom layout listens to that value and relays out whenever it notifies its
/// listeners, such as when an animation ticks. This allows the custom layout to avoid the build
/// phase of the pipeline.
///
/// Each child must be wrapped in a `LayoutId` widget to assign the id that identifies it to the
/// delegate. The id needs to be unique among the children that the `CustomMultiChildLayout`
/// manages.
pub trait MultiChildLayoutDelegate: Debug + 'static {
    /// The [`Listenable`] the layout will update on, Dart's `relayout` constructor argument.
    ///
    /// Defaults to `None`.
    fn relayout(&self) -> Option<&Rc<dyn Listenable>> {
        None
    }

    /// Override this method to return the size of this object given the incoming constraints.
    ///
    /// The size cannot reflect the sizes of the children. If this layout has a fixed width or
    /// height the returned size can reflect that; the size will be constrained to the given
    /// constraints.
    ///
    /// By default, attempts to size the box to the biggest size possible given the constraints.
    fn get_size(&self, constraints: BoxConstraints) -> Size {
        constraints.biggest()
    }

    /// Override this method to lay out and position all children given this widget's size.
    ///
    /// This method must call [`MultiChildLayoutChildren::layout_child`] for each child. It
    /// should also specify the final position of each child with
    /// [`MultiChildLayoutChildren::position_child`].
    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size);

    /// Override this method to return true when the children need to be laid out.
    ///
    /// This should compare the fields of the current delegate and the given `old_delegate` and
    /// return true if the fields are such that the layout would be different. `old_delegate` is
    /// of the same concrete type, Dart's `covariant`: narrow it with
    /// [`as_any`](Self::as_any).
    fn should_relayout(&self, old_delegate: &dyn MultiChildLayoutDelegate) -> bool;

    /// The concrete delegate, for Dart's `runtimeType` comparison and its `covariant`
    /// narrowing of [`should_relayout`](Self::should_relayout)'s argument.
    fn as_any(&self) -> &dyn Any;
}

/// Defers the layout of multiple children to a delegate.
///
/// The delegate can determine the layout constraints for each child and can decide where to
/// position each child. The delegate can also determine the size of the parent, but the size of
/// the parent cannot depend on the sizes of the children.
pub struct RenderCustomMultiChildLayoutBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    delegate: Rc<dyn MultiChildLayoutDelegate>,
}

impl RenderCustomMultiChildLayoutBox {
    /// Creates a render object that customizes the layout of multiple children.
    ///
    /// Add children with [`add`](ContainerRenderObjectMixin::add) or
    /// [`add_all`](ContainerRenderObjectMixin::add_all); Dart's constructor takes them.
    pub fn new(
        app: &mut App,
        delegate: Rc<dyn MultiChildLayoutDelegate>,
    ) -> RenderHandle<RenderCustomMultiChildLayoutBox> {
        RenderHandle::new_box(
            app,
            RenderCustomMultiChildLayoutBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                delegate,
            },
        )
    }

    /// The delegate that controls the layout of the children.
    pub fn delegate(self: RenderHandle<Self>, app: &App) -> Rc<dyn MultiChildLayoutDelegate> {
        Rc::clone(&self.get(app).delegate)
    }

    /// Sets [`delegate`](Self::delegate).
    pub fn set_delegate(
        self: RenderHandle<Self>,
        app: &mut App,
        new_delegate: Rc<dyn MultiChildLayoutDelegate>,
    ) {
        let old_delegate = Rc::clone(&self.get(app).delegate);
        if Rc::ptr_eq(&old_delegate, &new_delegate) {
            return;
        }
        if new_delegate.as_any().type_id() != old_delegate.as_any().type_id()
            || new_delegate.should_relayout(&*old_delegate)
        {
            self.mark_needs_layout(app);
        }
        self.get_mut(app).delegate = Rc::clone(&new_delegate);
        if self.attached(app) {
            if let Some(relayout) = old_delegate.relayout() {
                relayout.remove_listener(app, &self.relayout_listener());
            }
            if let Some(relayout) = new_delegate.relayout() {
                relayout.add_listener(app, self.relayout_listener());
            }
        }
    }

    /// The `markNeedsLayout` tear-off, equal to itself across registrations.
    fn relayout_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), mark_needs_layout)
    }

    /// Dart's `_getSize`.
    fn get_size(self: RenderHandle<Self>, app: &App, constraints: BoxConstraints) -> Size {
        debug_assert!(constraints.debug_assert_is_valid(false));
        constraints.constrain(self.get(app).delegate.get_size(constraints))
    }

    /// Dart's `MultiChildLayoutDelegate._callPerformLayout`.
    ///
    /// Dart saves and restores the delegate's `_idToChild` because a delegate can be used by
    /// both a parent and a child; here the map belongs to the call, so there is nothing to
    /// restore.
    fn call_perform_layout(self: RenderHandle<Self>, app: &mut App, size: Size) {
        let mut children = MultiChildLayoutChildren {
            id_to_child: HashMap::new(),
            #[cfg(debug_assertions)]
            children_needing_layout: std::collections::HashSet::new(),
        };
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of::<MultiChildLayoutParentData>(app);
            let (id, next_sibling) = (
                child_parent_data.id.clone(),
                child_parent_data.next_sibling(),
            );
            let id = id.unwrap_or_else(|| {
                panic!(
                    "Every child of a RenderCustomMultiChildLayoutBox must have an ID in its \
                     parent data. The following child has no ID: {current:?}"
                )
            });
            children.id_to_child.insert(id, current);
            #[cfg(debug_assertions)]
            children.children_needing_layout.insert(current);
            child = next_sibling;
        }

        let delegate = Rc::clone(&self.get(app).delegate);
        delegate.perform_layout(app, &mut children, size);

        #[cfg(debug_assertions)]
        assert!(
            children.children_needing_layout.is_empty(),
            "Each child must be laid out exactly once. The custom multichild layout delegate \
             forgot to lay out {:?}",
            children.children_needing_layout
        );
    }
}

fn mark_needs_layout(this: Handle<RenderCustomMultiChildLayoutBox>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_layout(app);
}

impl ContainerRenderObjectMixin for RenderCustomMultiChildLayoutBox {
    type ChildType = AnyRenderBox;
    type ParentDataType = MultiChildLayoutParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderCustomMultiChildLayoutBox {}

impl RenderObject for RenderCustomMultiChildLayoutBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.get_size(app, constraints);
        self.set_size(app, size);
        self.call_perform_layout(app, size);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
        if let Some(relayout) = self.delegate(app).relayout() {
            relayout.add_listener(app, self.relayout_listener());
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        if let Some(relayout) = self.delegate(app).relayout() {
            relayout.remove_listener(app, &self.relayout_listener());
        }
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        self.default_paint(app, context, offset);
    }
}

impl RenderBox for RenderCustomMultiChildLayoutBox {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<MultiChildLayoutParentData>(app) {
            child.set_parent_data(app, MultiChildLayoutParentData::new());
        }
    }

    // Dart's TODO stands: using the delegate's `getSize` for the intrinsic dimensions is a bit
    // dubious.

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let width = self
            .get_size(app, BoxConstraints::tight_for_finite(f64::INFINITY, height))
            .width();
        if width.is_finite() { width } else { 0.0 }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let width = self
            .get_size(app, BoxConstraints::tight_for_finite(f64::INFINITY, height))
            .width();
        if width.is_finite() { width } else { 0.0 }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let height = self
            .get_size(app, BoxConstraints::tight_for_finite(width, f64::INFINITY))
            .height();
        if height.is_finite() { height } else { 0.0 }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let height = self
            .get_size(app, BoxConstraints::tight_for_finite(width, f64::INFINITY))
            .height();
        if height.is_finite() { height } else { 0.0 }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.get_size(app, constraints)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        self.default_hit_test_children(app, result, position)
    }
}

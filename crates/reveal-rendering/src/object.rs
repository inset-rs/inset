//! Flutter counterpart: `rendering/object.dart` (`Constraints`, `ParentData`,
//! `RenderObject` tree / layout / paint marks). `PaintingContext` is in
//! `painting_context.rs`, `PipelineOwner` in `pipeline_owner.rs`.
//!
//! Semantics wait. A concrete node is [`RenderHandle<T>`] over the authored
//! struct. A link to another node is the type-erased handle [`AnyRenderObject`].
//! Authored methods take `self: RenderHandle<Self>`.

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Receiver;
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curve;
use reveal_embedder::{Matrix4, Offset, Rect};
use reveal_foundation::{App, Handle, HandleId};
use reveal_services::MouseTrackerAnnotation;

use crate::layer::{
    AnyContainerLayer, AnyOffsetLayer, ErasedLayer, LayerHandle, OffsetLayer, OffsetLayerMixin,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

thread_local! {
    static DEBUG_ACTIVE_LAYOUT: Cell<Option<AnyRenderObject>> = const { Cell::new(None) };
    static DEBUG_ACTIVE_PAINT: Cell<Option<AnyRenderObject>> = const { Cell::new(None) };
    static DEBUG_CHECKING_INTRINSICS: Cell<bool> = const { Cell::new(false) };
}

/// Whether the framework is currently checking a render object's intrinsic sizes, dry layout, or
/// dry baseline against its real layout.
///
/// Flutter's `RenderObject.debugCheckingIntrinsics`. While this is true, an intrinsic
/// computation neither caches its result nor throws where it cannot answer.
pub fn debug_checking_intrinsics() -> bool {
    DEBUG_CHECKING_INTRINSICS.get()
}

/// Sets [`debug_checking_intrinsics`].
pub fn set_debug_checking_intrinsics(value: bool) {
    DEBUG_CHECKING_INTRINSICS.set(value);
}

/// Immutable layout constraints.
///
/// Flutter's counterpart is the abstract `Constraints` class.
pub trait Constraints {
    /// Whether there is exactly one size possible given these constraints.
    fn is_tight(&self) -> bool;

    /// Whether the constraint is expressed in a consistent manner.
    fn is_normalized(&self) -> bool;

    /// Asserts that the constraints are valid.
    ///
    /// Returns the same as [`is_normalized`](Self::is_normalized) if asserts
    /// are disabled.
    fn debug_assert_is_valid(&self, is_applied_constraint: bool) -> bool {
        let _ = is_applied_constraint;
        debug_assert!(self.is_normalized());
        self.is_normalized()
    }
}

/// Data associated with a child by its parent.
///
/// Flutter's counterpart is the `ParentData` class. Subclasses are ordinary
/// Rust types that implement this trait.
pub trait ParentData: Any + Debug + Display {
    /// Called when the render object is removed from the tree.
    fn detach(&mut self) {}

    /// The half of this parent data whose type is `id`, if it embeds one.
    ///
    /// Dart's `parentData as BoxParentData` and `parentData is KeepAliveParentDataMixin`: a
    /// half answers for its own type, and a type that embeds one answers for its own type first
    /// and then asks the half, so a chain of halves composes. Every box parent must answer
    /// [`crate::BoxParentData`], or a child of it cannot be positioned by the box protocol.
    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        let _ = id;
        None
    }

    /// See [`provide`](Self::provide); mirrors it exactly.
    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        let _ = id;
        None
    }
}

impl dyn ParentData {
    /// The `P` half of this parent data: [`provide`](ParentData::provide), typed.
    pub fn part<P: 'static>(&self) -> Option<&P> {
        self.provide(TypeId::of::<P>())?.downcast_ref()
    }

    /// The `P` half of this parent data, mutably: [`provide_mut`](ParentData::provide_mut), typed.
    pub fn part_mut<P: 'static>(&mut self) -> Option<&mut P> {
        self.provide_mut(TypeId::of::<P>())?.downcast_mut()
    }
}

/// Flutter's `ParentData()` — no subclass fields.
#[derive(Debug, Default)]
pub struct EmptyParentData;

impl ParentData for EmptyParentData {}

impl Display for EmptyParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<none>")
    }
}

/// Flutter's `RenderObject` fields.
pub struct RenderObjectData {
    /// The erased vtable, recorded by the protocol constructor (`RenderHandle::new_box`,
    /// `RenderHandle::new_sliver`). Dart upcasts a subclass reference to `RenderObject` for
    /// free; here the fat pointer has to be built, and only the protocol knows the table.
    pub(crate) object_vtable: Option<&'static RenderObjectVTable>,
    pub(crate) parent: Option<AnyRenderObject>,
    pub(crate) owner: Option<Handle<PipelineOwner>>,
    pub(crate) depth: i32,
    pub(crate) parent_data: Option<Box<dyn ParentData>>,
    pub(crate) needs_layout: bool,
    pub(crate) needs_paint: bool,
    pub(crate) needs_composited_layer_update: bool,
    pub(crate) needs_compositing_bits_update: bool,
    pub(crate) needs_compositing: bool,
    pub(crate) was_repaint_boundary: bool,
    /// Flutter's `_layerHandle`.
    pub(crate) layer: LayerHandle<AnyContainerLayer>,
    pub(crate) is_relayout_boundary: Option<bool>,
    pub(crate) doing_this_layout_with_callback: bool,
    pub(crate) debug_doing_this_layout: bool,
    pub(crate) debug_doing_this_resize: bool,
    pub(crate) debug_doing_this_paint: bool,
    pub(crate) debug_can_parent_use_size: Option<bool>,
    pub(crate) debug_mutations_locked: bool,
}

impl RenderObjectData {
    /// Empty tree state for a newly constructed render object.
    pub fn new() -> RenderObjectData {
        RenderObjectData {
            object_vtable: None,
            parent: None,
            owner: None,
            depth: 0,
            parent_data: None,
            needs_layout: true,
            needs_paint: true,
            needs_composited_layer_update: false,
            needs_compositing_bits_update: false,
            needs_compositing: false,
            was_repaint_boundary: false,
            layer: LayerHandle::new(),
            is_relayout_boundary: None,
            doing_this_layout_with_callback: false,
            debug_doing_this_layout: false,
            debug_doing_this_resize: false,
            debug_doing_this_paint: false,
            debug_can_parent_use_size: None,
            debug_mutations_locked: false,
        }
    }
}

impl Default for RenderObjectData {
    fn default() -> RenderObjectData {
        RenderObjectData::new()
    }
}

/// Flutter's `RenderObjectWithChildMixin` child slot.
pub struct RenderObjectWithChildData<C> {
    pub(crate) child: Option<C>,
}

impl<C> RenderObjectWithChildData<C> {
    /// No child.
    pub const fn new() -> RenderObjectWithChildData<C> {
        RenderObjectWithChildData { child: None }
    }
}

impl<C> Default for RenderObjectWithChildData<C> {
    fn default() -> RenderObjectWithChildData<C> {
        RenderObjectWithChildData::new()
    }
}

/// Flutter's `ContainerParentDataMixin` fields: the links of a doubly-linked child list.
#[derive(Debug)]
pub struct ContainerParentData<ChildType> {
    pub(crate) previous_sibling: Option<ChildType>,
    pub(crate) next_sibling: Option<ChildType>,
}

impl<ChildType> ContainerParentData<ChildType> {
    /// No siblings.
    pub const fn new() -> ContainerParentData<ChildType> {
        ContainerParentData {
            previous_sibling: None,
            next_sibling: None,
        }
    }
}

impl<ChildType> Default for ContainerParentData<ChildType> {
    fn default() -> ContainerParentData<ChildType> {
        ContainerParentData::new()
    }
}

/// Parent data to support a doubly-linked list of children.
///
/// The children can be traversed using [`next_sibling`](Self::next_sibling) or
/// [`previous_sibling`](Self::previous_sibling), which can be called on the parent data of the
/// render objects obtained via [`crate::ContainerRenderObjectMixin::first_child`] or
/// [`crate::ContainerRenderObjectMixin::last_child`].
///
/// Flutter's `ContainerParentDataMixin`. The links live in a [`ContainerParentData`] field.
pub trait ContainerParentDataMixin: ParentData {
    /// The type-erased handle of a child in the list. `AnyRenderBox` for a box container.
    type ChildType: Copy + PartialEq;

    /// Mixin field access.
    fn container_parent_data(&self) -> &ContainerParentData<Self::ChildType>;

    /// See [`container_parent_data`](Self::container_parent_data).
    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<Self::ChildType>;

    /// The previous sibling in the parent's child list.
    fn previous_sibling(&self) -> Option<Self::ChildType> {
        self.container_parent_data().previous_sibling
    }

    /// Sets [`previous_sibling`](Self::previous_sibling).
    fn set_previous_sibling(&mut self, value: Option<Self::ChildType>) {
        self.container_parent_data_mut().previous_sibling = value;
    }

    /// The next sibling in the parent's child list.
    fn next_sibling(&self) -> Option<Self::ChildType> {
        self.container_parent_data().next_sibling
    }

    /// Sets [`next_sibling`](Self::next_sibling).
    fn set_next_sibling(&mut self, value: Option<Self::ChildType>) {
        self.container_parent_data_mut().next_sibling = value;
    }

    /// Clear the sibling pointers.
    ///
    /// The body of Flutter's `detach` override; call it from [`ParentData::detach`].
    fn detach(&mut self) {
        debug_assert!(
            self.previous_sibling().is_none(),
            "Pointers to siblings must be nulled before detaching ParentData."
        );
        debug_assert!(
            self.next_sibling().is_none(),
            "Pointers to siblings must be nulled before detaching ParentData."
        );
    }
}

/// Flutter's `ContainerRenderObjectMixin` fields: the head and tail of the child list, and its
/// length.
pub struct ContainerRenderObjectData<ChildType> {
    pub(crate) child_count: usize,
    pub(crate) first_child: Option<ChildType>,
    pub(crate) last_child: Option<ChildType>,
}

impl<ChildType> ContainerRenderObjectData<ChildType> {
    /// No children.
    pub const fn new() -> ContainerRenderObjectData<ChildType> {
        ContainerRenderObjectData {
            child_count: 0,
            first_child: None,
            last_child: None,
        }
    }
}

impl<ChildType> Default for ContainerRenderObjectData<ChildType> {
    fn default() -> ContainerRenderObjectData<ChildType> {
        ContainerRenderObjectData::new()
    }
}

/// An erased render object a parent can hold as a child: Dart's `ChildType extends RenderObject`
/// type argument of `RenderObjectWithChildMixin` and `ContainerRenderObjectMixin`.
///
/// Implemented by [`crate::AnyRenderBox`] and [`crate::AnyRenderSliver`].
pub trait ErasedRenderObject: Copy + PartialEq + Debug + 'static {
    /// The `RenderObject` view of this handle.
    fn as_object(self) -> AnyRenderObject;

    /// Dart's implicit downcast of a `RenderObject` to `ChildType`.
    ///
    /// # Panics
    ///
    /// If `object` does not use this protocol.
    fn from_object(object: AnyRenderObject) -> Self;
}

/// A render object with a single child.
///
/// Flutter's `RenderObjectWithChildMixin`. The child slot lives in a
/// [`RenderObjectWithChildData`] field.
pub trait RenderObjectWithChildMixin: RenderObject {
    /// Flutter's `ChildType`: the protocol of this render object's child.
    type ChildType: ErasedRenderObject;

    /// Mixin field access.
    fn child_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderObjectWithChildData<Self::ChildType>;

    /// See [`child_data`](Self::child_data).
    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<Self::ChildType>;

    /// The render object's unique child.
    fn child(self: RenderHandle<Self>, app: &App) -> Option<Self::ChildType> {
        self.child_data(app).child
    }

    /// Sets the unique child, adopting or dropping as Flutter's setter does.
    fn set_child(self: RenderHandle<Self>, app: &mut App, value: Option<Self::ChildType>) {
        if let Some(old) = self.child(app) {
            self.as_render_object(app).drop_child(app, old.as_object());
        }
        self.child_data_mut(app).child = value;
        if let Some(new) = value {
            self.as_render_object(app).adopt_child(app, new.as_object());
        }
    }
}

/// Generic mixin for render objects with a list of children.
///
/// Provides a child model for a render object that has a doubly-linked list of children.
///
/// [`ParentDataType`](ContainerRenderObjectMixin::ParentDataType) stores parent container data
/// on its child render objects. It must be a [`ContainerParentDataMixin`], which provides the
/// interface for visiting children. This data is populated by the `setup_parent_data`
/// implemented by the render object using this mixin.
///
/// Flutter's `ContainerRenderObjectMixin`. Its two type arguments are the associated types
/// [`ChildType`](ContainerRenderObjectMixin::ChildType) and
/// [`ParentDataType`](ContainerRenderObjectMixin::ParentDataType).
pub trait ContainerRenderObjectMixin: RenderObject {
    /// Flutter's `ChildType`: the protocol of this render object's children.
    type ChildType: ErasedRenderObject;

    /// Flutter's `ParentDataType`: the parent data this render object installs on its children.
    type ParentDataType: ContainerParentDataMixin<ChildType = Self::ChildType> + 'static;

    /// Mixin field access.
    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<Self::ChildType>;

    /// See [`container_data`](Self::container_data).
    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<Self::ChildType>;

    /// The number of children.
    fn child_count(self: RenderHandle<Self>, app: &App) -> usize {
        self.container_data(app).child_count
    }

    /// Insert child into this render object's child list after the given child.
    ///
    /// If `after` is `None`, then this inserts the child at the start of the list, and the child
    /// becomes the new [`first_child`](Self::first_child).
    fn insert(
        self: RenderHandle<Self>,
        app: &mut App,
        child: Self::ChildType,
        after: Option<Self::ChildType>,
    ) {
        ContainerRenderObjectBase::insert(self, app, child, after)
    }

    /// Append child to the end of this render object's child list.
    fn add(self: RenderHandle<Self>, app: &mut App, child: Self::ChildType) {
        let last_child = self.last_child(app);
        self.insert(app, child, last_child);
    }

    /// Add all the children to the end of this render object's child list.
    fn add_all(self: RenderHandle<Self>, app: &mut App, children: Option<Vec<Self::ChildType>>) {
        for child in children.into_iter().flatten() {
            self.add(app, child);
        }
    }

    /// Remove this child from the child list.
    ///
    /// Requires the child to be present in the child list.
    fn remove(self: RenderHandle<Self>, app: &mut App, child: Self::ChildType) {
        ContainerRenderObjectBase::remove(self, app, child)
    }

    /// Remove all their children from this render object's child list.
    ///
    /// More efficient than removing them individually.
    fn remove_all(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectBase::remove_all(self, app)
    }

    /// Move the given `child` in the child list to be after another child.
    ///
    /// Dart's `move`. More efficient than removing and re-adding the child. Requires the child
    /// to already be in the child list at some position. Pass `None` for `after` to move the
    /// child to the start of the child list.
    fn move_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: Self::ChildType,
        after: Option<Self::ChildType>,
    ) {
        ContainerRenderObjectBase::move_child(self, app, child, after)
    }

    /// The body of Flutter's `attach` override: attaches every child in the list.
    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            current.as_object().attach(app, owner);
            child = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app)
                .next_sibling();
        }
    }

    /// The body of Flutter's `detach` override: detaches every child in the list.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            current.as_object().detach(app);
            child = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app)
                .next_sibling();
        }
    }

    /// Flutter's `redepthChildren`: walks the child list.
    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            self.as_render_object(app)
                .redepth_child(app, current.as_object());
            child = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app)
                .next_sibling();
        }
    }

    /// Flutter's `visitChildren`: walks the child list.
    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            visitor(current.as_object());
            child = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app)
                .next_sibling();
        }
    }

    /// The first child in the child list.
    fn first_child(self: RenderHandle<Self>, app: &App) -> Option<Self::ChildType> {
        self.container_data(app).first_child
    }

    /// The last child in the child list.
    fn last_child(self: RenderHandle<Self>, app: &App) -> Option<Self::ChildType> {
        self.container_data(app).last_child
    }

    /// The previous child before the given child in the child list.
    fn child_before(
        self: RenderHandle<Self>,
        app: &App,
        child: Self::ChildType,
    ) -> Option<Self::ChildType> {
        debug_assert_eq!(
            child.as_object().parent(app),
            Some(self.as_render_object(app))
        );
        child
            .as_object()
            .parent_data_of::<Self::ParentDataType>(app)
            .previous_sibling()
    }

    /// The next child after the given child in the child list.
    fn child_after(
        self: RenderHandle<Self>,
        app: &App,
        child: Self::ChildType,
    ) -> Option<Self::ChildType> {
        debug_assert_eq!(
            child.as_object().parent(app),
            Some(self.as_render_object(app))
        );
        child
            .as_object()
            .parent_data_of::<Self::ParentDataType>(app)
            .next_sibling()
    }

    /// Returns a list containing the children of this render object.
    ///
    /// Dart's `RenderBoxContainerDefaultsMixin.getChildrenAsList`, useful for any container.
    fn children_as_list(self: RenderHandle<Self>, app: &App) -> Vec<Self::ChildType> {
        let mut result = Vec::new();
        let mut child = self.first_child(app);
        while let Some(current) = child {
            result.push(current);
            child = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app)
                .next_sibling();
        }
        result
    }
}

/// Flutter's `ContainerRenderObjectMixin` bodies that an override calls through `super`.
///
/// Blanket-implemented for every container, and repeats the default bodies of the four child-list
/// mutators: a leaf's own override shadows the default it would otherwise call.
pub trait ContainerRenderObjectBase: ContainerRenderObjectMixin {
    /// Flutter's `ContainerRenderObjectMixin.insert`.
    fn insert(
        self: RenderHandle<Self>,
        app: &mut App,
        child: Self::ChildType,
        after: Option<Self::ChildType>,
    ) {
        debug_assert!(
            child.as_object() != self.as_render_object(app),
            "A RenderObject cannot be inserted into itself."
        );
        debug_assert!(
            after.is_none_or(|after| after.as_object() != self.as_render_object(app)),
            "A RenderObject cannot simultaneously be both the parent and the sibling of another \
             RenderObject."
        );
        debug_assert!(
            after != Some(child),
            "A RenderObject cannot be inserted after itself."
        );
        debug_assert!(Some(child) != self.first_child(app));
        debug_assert!(Some(child) != self.last_child(app));
        self.as_render_object(app)
            .adopt_child(app, child.as_object());
        debug_assert!(
            child
                .as_object()
                .parent_data_is::<Self::ParentDataType>(app),
            "A child has parent data that does not conform to this render object's \
             ParentDataType. Override setup_parent_data to install it."
        );
        insert_into_child_list(self, app, child, after);
    }

    /// Flutter's `ContainerRenderObjectMixin.remove`.
    fn remove(self: RenderHandle<Self>, app: &mut App, child: Self::ChildType) {
        remove_from_child_list(self, app, child);
        self.as_render_object(app)
            .drop_child(app, child.as_object());
    }

    /// Flutter's `ContainerRenderObjectMixin.removeAll`.
    fn remove_all(self: RenderHandle<Self>, app: &mut App) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of_mut::<Self::ParentDataType>(app);
            let next = child_parent_data.next_sibling();
            child_parent_data.set_previous_sibling(None);
            child_parent_data.set_next_sibling(None);
            self.as_render_object(app)
                .drop_child(app, current.as_object());
            child = next;
        }
        let container = self.container_data_mut(app);
        container.first_child = None;
        container.last_child = None;
        container.child_count = 0;
    }

    /// Flutter's `ContainerRenderObjectMixin.move`.
    fn move_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: Self::ChildType,
        after: Option<Self::ChildType>,
    ) {
        debug_assert!(child.as_object() != self.as_render_object(app));
        debug_assert!(after.is_none_or(|after| after.as_object() != self.as_render_object(app)));
        debug_assert!(after != Some(child));
        debug_assert_eq!(
            child.as_object().parent(app),
            Some(self.as_render_object(app)),
            "the child must already be a child of this render object"
        );
        if child
            .as_object()
            .parent_data_of::<Self::ParentDataType>(app)
            .previous_sibling()
            == after
        {
            return;
        }
        remove_from_child_list(self, app, child);
        insert_into_child_list(self, app, child, after);
        self.as_render_object(app).mark_needs_layout(app);
    }
}

impl<T: ContainerRenderObjectMixin> ContainerRenderObjectBase for T {}

/// Flutter's `_debugUltimatePreviousSiblingOf`.
fn debug_ultimate_previous_sibling_of<P>(
    app: &App,
    child: P::ChildType,
    equals: Option<P::ChildType>,
) -> bool
where
    P: ContainerParentDataMixin + 'static,
    P::ChildType: ErasedRenderObject,
{
    let mut child = child;
    while let Some(previous) = child
        .as_object()
        .parent_data_of::<P>(app)
        .previous_sibling()
    {
        debug_assert!(previous != child);
        child = previous;
    }
    Some(child) == equals
}

/// Flutter's `_debugUltimateNextSiblingOf`.
fn debug_ultimate_next_sibling_of<P>(
    app: &App,
    child: P::ChildType,
    equals: Option<P::ChildType>,
) -> bool
where
    P: ContainerParentDataMixin + 'static,
    P::ChildType: ErasedRenderObject,
{
    let mut child = child;
    while let Some(next) = child.as_object().parent_data_of::<P>(app).next_sibling() {
        debug_assert!(next != child);
        child = next;
    }
    Some(child) == equals
}

/// Flutter's `_insertIntoChildList`.
fn insert_into_child_list<T: ContainerRenderObjectMixin>(
    this: RenderHandle<T>,
    app: &mut App,
    child: T::ChildType,
    after: Option<T::ChildType>,
) {
    type ParentDataOf<T> = <T as ContainerRenderObjectMixin>::ParentDataType;

    debug_assert!(
        child
            .as_object()
            .parent_data_of::<ParentDataOf<T>>(app)
            .next_sibling()
            .is_none()
    );
    debug_assert!(
        child
            .as_object()
            .parent_data_of::<ParentDataOf<T>>(app)
            .previous_sibling()
            .is_none()
    );
    this.container_data_mut(app).child_count += 1;
    let Some(after) = after else {
        // insert at the start (first_child)
        let first_child = this.first_child(app);
        child
            .as_object()
            .parent_data_of_mut::<ParentDataOf<T>>(app)
            .set_next_sibling(first_child);
        if let Some(first_child) = first_child {
            first_child
                .as_object()
                .parent_data_of_mut::<ParentDataOf<T>>(app)
                .set_previous_sibling(Some(child));
        }
        let container = this.container_data_mut(app);
        container.first_child = Some(child);
        container.last_child.get_or_insert(child);
        return;
    };
    debug_assert!(this.first_child(app).is_some());
    debug_assert!(this.last_child(app).is_some());
    debug_assert!(debug_ultimate_previous_sibling_of::<ParentDataOf<T>>(
        app,
        after,
        this.first_child(app)
    ));
    debug_assert!(debug_ultimate_next_sibling_of::<ParentDataOf<T>>(
        app,
        after,
        this.last_child(app)
    ));
    let after_next_sibling = after
        .as_object()
        .parent_data_of::<ParentDataOf<T>>(app)
        .next_sibling();
    let Some(after_next_sibling) = after_next_sibling else {
        // insert at the end (last_child); we'll end up with two or more children
        debug_assert_eq!(Some(after), this.last_child(app));
        child
            .as_object()
            .parent_data_of_mut::<ParentDataOf<T>>(app)
            .set_previous_sibling(Some(after));
        after
            .as_object()
            .parent_data_of_mut::<ParentDataOf<T>>(app)
            .set_next_sibling(Some(child));
        this.container_data_mut(app).last_child = Some(child);
        return;
    };
    // insert in the middle; we'll end up with three or more children
    // set up links from child to siblings
    let child_parent_data = child.as_object().parent_data_of_mut::<ParentDataOf<T>>(app);
    child_parent_data.set_next_sibling(Some(after_next_sibling));
    child_parent_data.set_previous_sibling(Some(after));
    // set up links from siblings to child
    after
        .as_object()
        .parent_data_of_mut::<ParentDataOf<T>>(app)
        .set_next_sibling(Some(child));
    after_next_sibling
        .as_object()
        .parent_data_of_mut::<ParentDataOf<T>>(app)
        .set_previous_sibling(Some(child));
}

/// Flutter's `_removeFromChildList`.
fn remove_from_child_list<T: ContainerRenderObjectMixin>(
    this: RenderHandle<T>,
    app: &mut App,
    child: T::ChildType,
) {
    type ParentDataOf<T> = <T as ContainerRenderObjectMixin>::ParentDataType;

    debug_assert!(debug_ultimate_previous_sibling_of::<ParentDataOf<T>>(
        app,
        child,
        this.first_child(app)
    ));
    debug_assert!(debug_ultimate_next_sibling_of::<ParentDataOf<T>>(
        app,
        child,
        this.last_child(app)
    ));
    let child_parent_data = child.as_object().parent_data_of::<ParentDataOf<T>>(app);
    let (previous_sibling, next_sibling) = (
        child_parent_data.previous_sibling(),
        child_parent_data.next_sibling(),
    );
    match previous_sibling {
        None => {
            debug_assert_eq!(this.first_child(app), Some(child));
            this.container_data_mut(app).first_child = next_sibling;
        }
        Some(previous_sibling) => previous_sibling
            .as_object()
            .parent_data_of_mut::<ParentDataOf<T>>(app)
            .set_next_sibling(next_sibling),
    }
    match next_sibling {
        None => {
            debug_assert_eq!(this.last_child(app), Some(child));
            this.container_data_mut(app).last_child = previous_sibling;
        }
        Some(next_sibling) => next_sibling
            .as_object()
            .parent_data_of_mut::<ParentDataOf<T>>(app)
            .set_previous_sibling(previous_sibling),
    }
    let child_parent_data = child.as_object().parent_data_of_mut::<ParentDataOf<T>>(app);
    child_parent_data.set_previous_sibling(None);
    child_parent_data.set_next_sibling(None);
    this.container_data_mut(app).child_count -= 1;
}

/// Class-specific render object: the data accessors and the virtuals, nothing else. Methods
/// take [`RenderHandle<Self>`] so they can re-enter the same slot through [`App`].
///
/// Tree operations (`mark_needs_layout`, `adopt_child`, `parent`, …) live on the erased
/// [`AnyRenderObject`]; a typed handle reaches them through its protocol trait,
/// [`crate::RenderBox`] or [`crate::RenderSliver`].
pub trait RenderObject: 'static + Sized {
    /// Mixin field access.
    fn render_object_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectData;

    /// See [`render_object_data`](Self::render_object_data).
    fn render_object_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderObjectData;

    /// Dart's implicit upcast of a concrete render object to `RenderObject`: this object's
    /// type-erased handle, through the vtable the protocol recorded when it created it.
    fn as_render_object(self: RenderHandle<Self>, app: &App) -> AnyRenderObject {
        let vtable = self
            .render_object_data(app)
            .object_vtable
            .expect("a render object records its vtable when the protocol creates it");
        AnyRenderObject::from_vtable(self.id(), vtable)
    }

    /// Do the work of computing the layout for this render object.
    ///
    /// Do not call this function directly: call `layout` on the protocol handle instead.
    /// Read the constraints with `constraints` on the protocol trait.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App);

    /// Updates the render object's size using only the constraints.
    ///
    /// Called by `layout` only when [`sized_by_parent`](Self::sized_by_parent) is true.
    ///
    /// A box overrides [`crate::RenderBox::compute_dry_layout`] instead: the box protocol's
    /// [`crate::RenderBox::perform_resize`] sizes itself from it.
    fn perform_resize(self: RenderHandle<Self>, _app: &mut App) {
        let _ = self;
        panic!("RenderObject::perform_resize was called; override it when sized_by_parent is true")
    }

    /// Calls `visitor` for each immediate child.
    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        let _ = (self, app, visitor);
    }

    /// Sets up parent data for `child` if it has none.
    ///
    /// For a box, override [`crate::RenderBox::setup_parent_data`] instead; an implementation
    /// here is not called for boxes.
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if child.parent_data(app).is_none() {
            child.set_parent_data(app, EmptyParentData);
        }
    }

    /// The body of Flutter's `attach` override after `super.attach(owner)`; the base body has
    /// already run. The default attaches the children reported by
    /// [`visit_children`](Self::visit_children).
    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        for child in collect_children(app, self) {
            child.attach(app, owner);
        }
    }

    /// The body of Flutter's `detach` override after `super.detach()`. The default detaches
    /// the children reported by [`visit_children`](Self::visit_children).
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        for child in collect_children(app, self) {
            child.detach(app);
        }
    }

    /// Adjusts the depth of each child reported by [`visit_children`](Self::visit_children).
    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        let depth = self.render_object_data(app).depth;
        let owner = self.render_object_data(app).owner;
        for child in collect_children(app, self) {
            debug_assert_eq!(child.owner(app), owner);
            child.redepth_below(app, depth);
        }
    }

    /// Whether the constraints are the only input to the sizing algorithm.
    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        let _ = self;
        false
    }

    /// Applies the transform that would be applied when painting the given child to the
    /// given matrix.
    ///
    /// Used by coordinate conversion functions ([`AnyRenderObject::get_transform_to`]) to
    /// translate coordinates local to one render object into coordinates local to another
    /// render object.
    ///
    /// Some RenderObjects will provide a zeroed out matrix in this method, indicating that
    /// the child should not paint anything or respond to hit tests currently. A parent may
    /// supply a non-zero matrix even if it does not paint its child currently, for example
    /// if the parent is a `RenderOffstage` with `offstage` set to true. In both of these
    /// cases, the parent must return `false` from `paintsChild`.
    ///
    /// The box protocol's default translates by the child's `BoxParentData` offset; this
    /// object-level default only asserts, as Dart's `RenderObject.applyPaintTransform`.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let _ = transform;
        debug_assert!(child.parent(app).map(AnyRenderObject::id) == Some(self.id()));
    }

    /// Whether this render object repaints separately from its parent.
    ///
    /// Override this in subclasses to indicate that instances of your class ought
    /// to repaint independently. For example, render objects that repaint
    /// frequently might want to repaint themselves without requiring their parent
    /// to repaint.
    ///
    /// If this getter returns true, the [`paint_bounds`](Self::paint_bounds) are applied to this
    /// object and all descendants. The framework invokes
    /// [`update_composited_layer`](Self::update_composited_layer) to create an [`OffsetLayer`]
    /// and assigns it to the [`layer`](AnyRenderObject::layer) field.
    /// Render objects that declare themselves as repaint boundaries must not replace
    /// the layer created by the framework.
    ///
    /// If the value of this getter changes, [`AnyRenderObject::mark_needs_compositing_bits_update`]
    /// must be called.
    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        let _ = self;
        false
    }

    /// Whether this render object always needs compositing.
    ///
    /// Override this in subclasses to indicate that your paint function always
    /// creates at least one composited layer. For example, videos should return
    /// true if they use hardware decoders.
    ///
    /// You must call [`AnyRenderObject::mark_needs_compositing_bits_update`] if the value of this
    /// getter changes. (This is implied when [`AnyRenderObject::adopt_child`] or
    /// [`AnyRenderObject::drop_child`] are called.)
    fn always_needs_compositing(self: RenderHandle<Self>, _app: &App) -> bool {
        let _ = self;
        false
    }

    /// Dart's `this is MouseTrackerAnnotation`: the annotation this render object implements,
    /// read live each time the mouse tracker consults it. `None` for a render object that is
    /// not an annotation, which is the default.
    fn mouse_tracker_annotation(
        self: RenderHandle<Self>,
        _app: &App,
    ) -> Option<MouseTrackerAnnotation> {
        let _ = self;
        None
    }

    /// Update the composited layer owned by this render object.
    ///
    /// This method is called by the framework when [`is_repaint_boundary`](Self::is_repaint_boundary)
    /// is true.
    ///
    /// If `old_layer` is `None`, this method must return a new [`OffsetLayer`]
    /// (or subtype thereof). If `old_layer` is not `None`, then this method must
    /// reuse the layer instance that is provided — it is an error to create a new
    /// layer in this instance. The layer will be disposed by the framework when
    /// either the render object is disposed or if it is no longer a repaint
    /// boundary.
    ///
    /// The [`OffsetLayerMixin::offset`](crate::OffsetLayerMixin::offset) property will be managed
    /// by the framework and must not be updated by this method.
    ///
    /// If a property of the composited layer needs to be updated, the render object
    /// must call [`AnyRenderObject::mark_needs_composited_layer_update`] which will schedule this
    /// method to be called without repainting children. If this widget was marked as
    /// needing to paint and needing a composited layer update, this method is only
    /// called once.
    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<AnyOffsetLayer>,
    ) -> AnyOffsetLayer {
        debug_assert!(self.is_repaint_boundary(app));
        old_layer.unwrap_or_else(|| OffsetLayer::new(app, Offset::ZERO).as_offset_layer())
    }

    /// Attempt to make (a portion of) this or a descendant [`RenderObject`] visible on screen.
    ///
    /// If `descendant` is provided, that render object is made visible. If `descendant` is
    /// omitted, this render object is made visible.
    ///
    /// The optional `rect` parameter describes which area of that render object should be
    /// shown on screen. If `rect` is `None`, the entire render object (as defined by its
    /// paint bounds) will be revealed. The `rect` parameter is interpreted relative to the
    /// coordinate system of `descendant` if that argument is provided and relative to this
    /// render object otherwise.
    ///
    /// The `duration` parameter can be set to a non-zero value to bring the target object on
    /// screen in an animation defined by `curve`.
    ///
    /// See also:
    ///
    /// * [`crate::show_in_viewport`], which `RenderViewportBase` delegates
    ///   this method to.
    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        RenderObjectBase::show_on_screen(self, app, descendant, rect, duration, curve)
    }

    /// Dart's `object is RenderAbstractViewport` for any interface: the erased handle of the
    /// interface `id` names, if this type implements it.
    ///
    /// An implementor answers each interface it implements with that interface's type-erased
    /// handle, boxed; [`AnyRenderObject::interface`] unboxes it. The default implements none.
    fn interface(self: RenderHandle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        let _ = (self, id);
        None
    }

    /// Release any resources held by this render object.
    ///
    /// The object that creates a [`RenderObject`] is in charge of disposing it. If this render
    /// object has created any children directly, it must dispose of those children in this method
    /// as well. It must not dispose of any children that were created by some other object, such
    /// as a `RenderObjectElement`. Those children will be disposed when that element unmounts,
    /// which may be delayed if the element is moved to another part of the tree.
    ///
    /// Implementations of this method must end with a call to the inherited method,
    /// `RenderObjectBase::dispose(self, app)`, as in Dart's `super.dispose()`.
    ///
    /// The object is no longer usable after calling dispose. The arena has no collector, so
    /// [`AnyRenderObject::dispose`] destroys the slot after this hook returns.
    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        RenderObjectBase::dispose(self, app);
    }

    /// Paint this render object into the given context at the given offset.
    ///
    /// Do not call this function directly: [`PaintingContext::paint_child`] does. Paint children
    /// with `context.paint_child(app, child, offset + child_offset)`; do not use the canvas to
    /// translate, and do not use it after painting a child.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let _ = (self, app, context, offset);
    }
}

fn collect_children<T: RenderObject>(app: &App, this: RenderHandle<T>) -> Vec<AnyRenderObject> {
    let mut children = Vec::new();
    this.visit_children(app, &mut |child| children.push(child));
    children
}

/// Implements the [`RenderObject`] field accessors for a `render_object` field.
#[macro_export]
macro_rules! render_object_accessors {
    () => {
        fn render_object_data(
            self: $crate::RenderHandle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RenderObjectData {
            &self.get(app).render_object
        }
        fn render_object_data_mut(
            self: $crate::RenderHandle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RenderObjectData {
            &mut self.get_mut(app).render_object
        }
    };
}

/// Typed access to a concrete render object's class-specific state.
///
/// [`get`](Self::get) / [`get_mut`](Self::get_mut) are point access: drop the
/// borrow before any call that can re-enter this slot.
pub struct RenderHandle<T>(Handle<T>);

impl<T> Clone for RenderHandle<T> {
    fn clone(&self) -> RenderHandle<T> {
        *self
    }
}

impl<T> Copy for RenderHandle<T> {}

impl<T> Receiver for RenderHandle<T> {
    type Target = T;
}

impl<T> PartialEq for RenderHandle<T> {
    fn eq(&self, other: &RenderHandle<T>) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for RenderHandle<T> {}

impl<T> Hash for RenderHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> Debug for RenderHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RenderHandle<{}>({:?})",
            std::any::type_name::<T>(),
            self.0.id()
        )
    }
}

impl<T: 'static> RenderHandle<T> {
    /// The concrete state. Drop the borrow before a virtual call.
    pub fn get(self, app: &App) -> &T {
        app.get(self.0)
    }

    /// See [`get`](Self::get).
    pub fn get_mut(self, app: &mut App) -> &mut T {
        app.get_mut(self.0)
    }

    pub(crate) fn id(self) -> HandleId {
        self.0.id()
    }

    /// The foundation handle, for a [`reveal_foundation::Listener::handle_method`] tear-off.
    pub fn handle(self) -> Handle<T> {
        self.0
    }

    /// The render handle for a foundation handle, which is what such a tear-off is called with.
    pub fn from_handle(handle: Handle<T>) -> RenderHandle<T> {
        RenderHandle(handle)
    }
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
pub(crate) fn resolve<T: 'static>(id: HandleId) -> RenderHandle<T> {
    RenderHandle(Handle::from_id(id))
}

pub(crate) fn create<T: 'static>(app: &mut App, object: T) -> RenderHandle<T> {
    RenderHandle(app.create(object))
}

/// The dispatch signature of [`RenderObject::show_on_screen`].
pub(crate) type ShowOnScreenFn =
    fn(&mut App, HandleId, Option<AnyRenderObject>, Option<Rect>, Duration, Rc<dyn Curve>);

/// The vtable of an erased [`AnyRenderObject`]: one `&'static` table per type, built from the
/// trait impl by the protocol. Copied out of the handle before a virtual call, so the slot is not
/// borrowed across it.
pub(crate) struct RenderObjectVTable {
    pub object_data: fn(&App, HandleId) -> &RenderObjectData,
    pub object_data_mut: fn(&mut App, HandleId) -> &mut RenderObjectData,
    pub visit_children: fn(&App, HandleId, &mut dyn FnMut(AnyRenderObject)),
    pub did_attach: fn(&mut App, HandleId, Handle<PipelineOwner>),
    pub did_detach: fn(&mut App, HandleId),
    pub redepth_children: fn(&mut App, HandleId),
    pub setup_parent_data: fn(&mut App, HandleId, AnyRenderObject),
    pub sized_by_parent: fn(&App, HandleId) -> bool,
    pub perform_layout: fn(&mut App, HandleId),
    pub perform_resize: fn(&mut App, HandleId),
    pub mark_needs_layout: fn(&mut App, HandleId),
    pub paint_bounds: fn(&App, HandleId) -> Rect,
    pub apply_paint_transform: fn(&App, HandleId, AnyRenderObject, &mut Matrix4),
    pub is_repaint_boundary: fn(&App, HandleId) -> bool,
    pub always_needs_compositing: fn(&App, HandleId) -> bool,
    pub mouse_tracker_annotation: fn(&App, HandleId) -> Option<MouseTrackerAnnotation>,
    pub update_composited_layer: fn(&mut App, HandleId, Option<AnyOffsetLayer>) -> AnyOffsetLayer,
    pub show_on_screen: ShowOnScreenFn,
    pub interface: fn(HandleId, TypeId) -> Option<Box<dyn Any>>,
    pub paint: fn(&mut App, HandleId, &mut PaintingContext, Offset),
    pub dispose: fn(&mut App, HandleId),
    /// The protocol table this object table is nested in. Exactly one is `Some`.
    pub as_box: Option<fn() -> &'static crate::box_::RenderBoxVTable>,
    pub as_sliver: Option<fn() -> &'static crate::sliver::RenderSliverVTable>,
}

impl RenderObjectVTable {
    /// `setup_parent_data`, `apply_paint_transform`, `perform_resize` and `mark_needs_layout`
    /// are passed in because the box protocol has its own defaults; `paint_bounds` because each
    /// protocol defines it.
    pub(crate) const fn of<T: RenderObject>(
        setup_parent_data: fn(&mut App, HandleId, AnyRenderObject),
        paint_bounds: fn(&App, HandleId) -> Rect,
        apply_paint_transform: fn(&App, HandleId, AnyRenderObject, &mut Matrix4),
        perform_resize: fn(&mut App, HandleId),
        mark_needs_layout: fn(&mut App, HandleId),
        as_box: Option<fn() -> &'static crate::box_::RenderBoxVTable>,
        as_sliver: Option<fn() -> &'static crate::sliver::RenderSliverVTable>,
    ) -> RenderObjectVTable {
        RenderObjectVTable {
            object_data: |app, id| T::render_object_data(resolve(id), app),
            object_data_mut: |app, id| T::render_object_data_mut(resolve(id), app),
            visit_children: |app, id, visitor| T::visit_children(resolve(id), app, visitor),
            did_attach: |app, id, owner| T::did_attach(resolve(id), app, owner),
            did_detach: |app, id| T::did_detach(resolve(id), app),
            redepth_children: |app, id| T::redepth_children(resolve(id), app),
            setup_parent_data,
            sized_by_parent: |app, id| T::sized_by_parent(resolve(id), app),
            perform_layout: |app, id| T::perform_layout(resolve(id), app),
            perform_resize,
            mark_needs_layout,
            paint_bounds,
            apply_paint_transform,
            is_repaint_boundary: |app, id| T::is_repaint_boundary(resolve(id), app),
            always_needs_compositing: |app, id| T::always_needs_compositing(resolve(id), app),
            mouse_tracker_annotation: |app, id| T::mouse_tracker_annotation(resolve(id), app),
            update_composited_layer: |app, id, old_layer| {
                T::update_composited_layer(resolve(id), app, old_layer)
            },
            show_on_screen: |app, id, descendant, rect, duration, curve| {
                T::show_on_screen(resolve(id), app, descendant, rect, duration, curve)
            },
            interface: |id, interface| T::interface(resolve(id), interface),
            paint: |app, id, context, offset| T::paint(resolve(id), app, context, offset),
            dispose: |app, id| T::dispose(resolve(id), app),
            as_box,
            as_sliver,
        }
    }
}

/// Erased `RenderObject`: one identity and a static vtable, the fat pointer rustc cannot build
/// for an arena id. No lease.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyRenderObject {
    id: HandleId,
    vtable: &'static RenderObjectVTable,
}

impl PartialEq for AnyRenderObject {
    fn eq(&self, other: &AnyRenderObject) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRenderObject {}

impl Hash for AnyRenderObject {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<AnyRenderObject> for HandleId {
    fn from(object: AnyRenderObject) -> HandleId {
        object.id
    }
}

impl Debug for AnyRenderObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyRenderObject({:?})", self.id)
    }
}

impl AnyRenderObject {
    pub(crate) fn from_vtable(
        id: HandleId,
        vtable: &'static RenderObjectVTable,
    ) -> AnyRenderObject {
        AnyRenderObject { id, vtable }
    }

    /// Dart's `object as RenderBox`, as an `Option`.
    pub fn as_box(self) -> Option<crate::box_::AnyRenderBox> {
        self.vtable
            .as_box
            .map(|vtable| crate::box_::AnyRenderBox::from_vtable(self.id, vtable()))
    }

    /// Dart's `object as RenderSliver`, as an `Option`.
    pub fn as_sliver(self) -> Option<crate::sliver::AnyRenderSliver> {
        self.vtable
            .as_sliver
            .map(|vtable| crate::sliver::AnyRenderSliver::from_vtable(self.id, vtable()))
    }

    /// The render object that is actively computing layout.
    ///
    /// Only meaningful when asserts are enabled.
    pub fn debug_active_layout() -> Option<AnyRenderObject> {
        DEBUG_ACTIVE_LAYOUT.get()
    }

    pub(crate) fn set_debug_active_layout(node: Option<AnyRenderObject>) {
        DEBUG_ACTIVE_LAYOUT.set(node);
    }

    /// The render object that is actively painting.
    ///
    /// Only meaningful when asserts are enabled.
    pub fn debug_active_paint() -> Option<AnyRenderObject> {
        DEBUG_ACTIVE_PAINT.get()
    }

    fn data(self, app: &App) -> &RenderObjectData {
        (self.vtable.object_data)(app, self.id)
    }

    pub(crate) fn data_mut(self, app: &mut App) -> &mut RenderObjectData {
        (self.vtable.object_data_mut)(app, self.id)
    }

    /// Data for use by the parent render object.
    pub fn parent_data(self, app: &App) -> Option<&dyn ParentData> {
        self.data(app).parent_data.as_deref()
    }

    /// Mutable parent data.
    pub fn parent_data_mut(self, app: &mut App) -> Option<&mut dyn ParentData> {
        self.data_mut(app).parent_data.as_deref_mut()
    }

    /// Whether parent data is `P`. Dart's `parentData is P`.
    pub fn parent_data_is<P: ParentData + 'static>(self, app: &App) -> bool {
        self.parent_data(app)
            .is_some_and(|data| (data as &dyn Any).is::<P>())
    }

    /// Dart's `parentData! as P`.
    ///
    /// # Panics
    ///
    /// If parent data is missing or not `P`.
    pub fn parent_data_of<P: ParentData + 'static>(self, app: &App) -> &P {
        self.parent_data(app)
            .and_then(|data| (data as &dyn Any).downcast_ref::<P>())
            .unwrap_or_else(|| panic!("parent data is not {}", std::any::type_name::<P>()))
    }

    /// See [`parent_data_of`](Self::parent_data_of).
    pub fn parent_data_of_mut<P: ParentData + 'static>(self, app: &mut App) -> &mut P {
        self.parent_data_mut(app)
            .and_then(|data| (data as &mut dyn Any).downcast_mut::<P>())
            .unwrap_or_else(|| panic!("parent data is not {}", std::any::type_name::<P>()))
    }

    /// Installs parent data. Dart's `child.parentData = …`.
    pub fn set_parent_data(self, app: &mut App, parent_data: impl ParentData + 'static) {
        self.data_mut(app).parent_data = Some(Box::new(parent_data));
    }

    fn clear_parent_data(self, app: &mut App) {
        self.data_mut(app).parent_data = None;
    }

    /// The parent of this render object in the render tree.
    pub fn parent(self, app: &App) -> Option<AnyRenderObject> {
        self.data(app).parent
    }

    fn set_parent(self, app: &mut App, parent: Option<AnyRenderObject>) {
        self.data_mut(app).parent = parent;
    }

    /// The owner for this render object (`None` if unattached).
    pub fn owner(self, app: &App) -> Option<Handle<PipelineOwner>> {
        self.data(app).owner
    }

    /// Whether this render object belongs to a [`PipelineOwner`].
    pub fn attached(self, app: &App) -> bool {
        self.owner(app).is_some()
    }

    /// The depth of this render object in the render tree.
    pub fn depth(self, app: &App) -> i32 {
        self.data(app).depth
    }

    fn set_depth(self, app: &mut App, depth: i32) {
        self.data_mut(app).depth = depth;
    }

    pub(crate) fn needs_layout(self, app: &App) -> bool {
        self.data(app).needs_layout
    }

    fn set_needs_layout(self, app: &mut App, value: bool) {
        self.data_mut(app).needs_layout = value;
    }

    /// Whether this render object's layout information is dirty.
    ///
    /// Always `false` when debug assertions are disabled.
    pub fn debug_needs_layout(self, app: &App) -> bool {
        if !cfg!(debug_assertions) {
            return false;
        }
        self.needs_layout(app)
    }

    pub(crate) fn is_relayout_boundary(self, app: &App) -> Option<bool> {
        self.data(app).is_relayout_boundary
    }

    fn set_is_relayout_boundary(self, app: &mut App, value: Option<bool>) {
        self.data_mut(app).is_relayout_boundary = value;
    }

    fn doing_this_layout_with_callback(self, app: &App) -> bool {
        self.data(app).doing_this_layout_with_callback
    }

    pub(crate) fn debug_doing_this_layout(self, app: &App) -> bool {
        self.data(app).debug_doing_this_layout
    }

    fn set_debug_doing_this_layout(self, app: &mut App, value: bool) {
        self.data_mut(app).debug_doing_this_layout = value;
    }

    pub(crate) fn debug_doing_this_resize(self, app: &App) -> bool {
        self.data(app).debug_doing_this_resize
    }

    fn set_debug_doing_this_resize(self, app: &mut App, value: bool) {
        self.data_mut(app).debug_doing_this_resize = value;
    }

    fn set_debug_can_parent_use_size(self, app: &mut App, value: Option<bool>) {
        self.data_mut(app).debug_can_parent_use_size = value;
    }

    fn debug_mutations_locked(self, app: &App) -> bool {
        self.data(app).debug_mutations_locked
    }

    fn set_debug_mutations_locked(self, app: &mut App, value: bool) {
        self.data_mut(app).debug_mutations_locked = value;
    }

    fn sized_by_parent(self, app: &App) -> bool {
        (self.vtable.sized_by_parent)(app, self.id)
    }

    /// Calls `visitor` for each immediate child.
    pub fn visit_children(self, app: &App, visitor: &mut dyn FnMut(AnyRenderObject)) {
        (self.vtable.visit_children)(app, self.id, visitor)
    }

    /// Override point used by [`adopt_child`](Self::adopt_child).
    pub fn setup_parent_data(self, app: &mut App, child: AnyRenderObject) {
        (self.vtable.setup_parent_data)(app, self.id, child)
    }

    /// Adjust the depth of `child` to be greater than this node's own depth.
    pub fn redepth_child(self, app: &mut App, child: AnyRenderObject) {
        debug_assert_eq!(child.owner(app), self.owner(app));
        child.redepth_below(app, self.depth(app));
    }

    /// Flutter's `redepthChild` from the child's side: `parent_depth` is the parent's depth.
    pub(crate) fn redepth_below(self, app: &mut App, parent_depth: i32) {
        if self.depth(app) <= parent_depth {
            self.set_depth(app, parent_depth + 1);
            (self.vtable.redepth_children)(app, self.id);
        }
    }

    /// Called by subclasses when they decide a render object is a child.
    pub fn adopt_child(self, app: &mut App, child: AnyRenderObject) {
        debug_assert!(child.parent(app).is_none());
        if cfg!(debug_assertions) {
            let mut node = Some(self);
            while let Some(current) = node {
                debug_assert_ne!(current, child, "adopt_child would create a cycle");
                node = current.parent(app);
            }
        }
        self.setup_parent_data(app, child);
        self.mark_needs_layout(app);
        self.mark_needs_compositing_bits_update(app);
        child.set_parent(app, Some(self));
        if self.attached(app) {
            child.attach(app, self.owner(app).expect("attached parent has an owner"));
        }
        self.redepth_child(app, child);
    }

    /// Called by subclasses when they decide a render object is no longer a child.
    pub fn drop_child(self, app: &mut App, child: AnyRenderObject) {
        debug_assert_eq!(child.parent(app), Some(self));
        debug_assert_eq!(child.attached(app), self.attached(app));
        debug_assert!(child.parent_data(app).is_some());
        if !child.is_relayout_boundary(app).unwrap_or(true) {
            child.set_is_relayout_boundary(app, None);
        }
        if let Some(parent_data) = child.parent_data_mut(app) {
            parent_data.detach();
        }
        child.clear_parent_data(app);
        child.set_parent(app, None);
        if self.attached(app) {
            child.detach(app);
        }
        self.mark_needs_layout(app);
        self.mark_needs_compositing_bits_update(app);
    }

    /// Mark this render object as attached to `owner`.
    ///
    /// Flutter's `RenderObject.attach` body, then the [`RenderObject::did_attach`] hook, which
    /// by default attaches the children.
    pub fn attach(self, app: &mut App, owner: Handle<PipelineOwner>) {
        debug_assert!(
            !app.is_disposed(self.id),
            "attach on a disposed render object"
        );
        debug_assert!(self.owner(app).is_none());
        self.data_mut(app).owner = Some(owner);
        if self.needs_layout(app) && self.is_relayout_boundary(app).is_some() {
            self.set_needs_layout(app, false);
            self.mark_needs_layout(app);
        }
        if self.needs_compositing_bits_update(app) {
            self.set_needs_compositing_bits_update(app, false);
            self.mark_needs_compositing_bits_update(app);
        }
        if self.needs_paint(app) && self.layer(app).is_some() {
            self.set_needs_paint(app, false);
            self.mark_needs_paint(app);
        }
        (self.vtable.did_attach)(app, self.id, owner)
    }

    /// Mark this render object as detached from its [`PipelineOwner`].
    ///
    /// Flutter's `RenderObject.detach` body, then the [`RenderObject::did_detach`] hook, which
    /// by default detaches the children.
    pub fn detach(self, app: &mut App) {
        debug_assert!(self.owner(app).is_some());
        self.data_mut(app).owner = None;
        debug_assert!(
            self.parent(app).is_none()
                || self.attached(app)
                    == self.parent(app).is_some_and(|parent| parent.attached(app))
        );
        (self.vtable.did_detach)(app, self.id)
    }

    /// Mark this render object's layout information as dirty.
    ///
    /// The box protocol overrides this with [`crate::RenderBox::mark_needs_layout`], which
    /// clears the intrinsics and dry-layout caches first.
    pub fn mark_needs_layout(self, app: &mut App) {
        debug_assert!(
            !app.is_disposed(self.id),
            "mark_needs_layout on a disposed render object"
        );
        (self.vtable.mark_needs_layout)(app, self.id)
    }

    /// Mark this render object's layout information as dirty, and then defer to
    /// the parent.
    pub fn mark_parent_needs_layout(self, app: &mut App) {
        self.set_needs_layout(app, true);
        let parent = self
            .parent(app)
            .expect("mark_parent_needs_layout requires a parent");
        if !self.doing_this_layout_with_callback(app) {
            parent.mark_needs_layout(app);
        } else {
            debug_assert!(parent.debug_doing_this_layout(app));
        }
        debug_assert_eq!(Some(parent), self.parent(app));
    }

    /// Mark layout dirty and also mark the parent, for a `sized_by_parent` change.
    /// Allows mutations to be made to this object's child list (and any descendants) as
    /// well as to any other dirty nodes in the render tree owned by the same `PipelineOwner`
    /// as this object. The `callback` argument is invoked synchronously, and the mutations
    /// are allowed only during that callback's execution.
    ///
    /// This exists to allow child lists to be built on-demand during layout (e.g. based on
    /// the object's size), and to enable nodes to be moved around the tree as this happens
    /// (e.g. to handle `GlobalKey` reparenting), while still ensuring that any particular
    /// node is only laid out once per frame.
    ///
    /// Calling this function disables a number of asserts that are intended to catch likely
    /// bugs. As such, using this function is generally discouraged.
    ///
    /// This function can only be called during layout.
    pub fn invoke_layout_callback(self, app: &mut App, callback: impl FnOnce(&mut App)) {
        debug_assert!(self.debug_mutations_locked(app));
        debug_assert!(self.debug_doing_this_layout(app));
        debug_assert!(!self.doing_this_layout_with_callback(app));
        self.data_mut(app).doing_this_layout_with_callback = true;
        let owner = self.owner(app).expect("a laying-out object is attached");
        owner.enable_mutations_to_dirty_subtrees(app, callback);
        self.data_mut(app).doing_this_layout_with_callback = false;
    }

    pub fn mark_needs_layout_for_sized_by_parent_change(self, app: &mut App) {
        self.mark_needs_layout(app);
        self.mark_parent_needs_layout(app);
    }

    /// Bootstrap the rendering pipeline by scheduling the very first layout.
    pub fn schedule_initial_layout(self, app: &mut App) {
        debug_assert!(self.attached(app));
        debug_assert!(self.parent(app).is_none());
        debug_assert!(!self.owner(app).expect("attached").debug_doing_layout(app));
        debug_assert!(self.is_relayout_boundary(app).is_none());
        self.set_is_relayout_boundary(app, Some(true));
        if cfg!(debug_assertions) {
            self.set_debug_can_parent_use_size(app, Some(false));
        }
        let owner = self.owner(app).expect("attached");
        owner.add_node_needing_layout(app, self);
    }

    pub(crate) fn layout_without_resize(self, app: &mut App) {
        debug_assert!(self.needs_layout(app));
        debug_assert!(self.is_relayout_boundary(app).unwrap_or(false));
        let perform_layout = self.vtable.perform_layout;
        let previous = if cfg!(debug_assertions) {
            debug_assert!(!self.doing_this_layout_with_callback(app));
            self.set_debug_mutations_locked(app, true);
            self.set_debug_doing_this_layout(app, true);
            let previous = AnyRenderObject::debug_active_layout();
            AnyRenderObject::set_debug_active_layout(Some(self));
            if crate::debug::debug_print_layouts() {
                eprintln!("Laying out (without resize) {self:?}");
            }
            previous
        } else {
            None
        };
        perform_layout(app, self.id);
        if cfg!(debug_assertions) {
            AnyRenderObject::set_debug_active_layout(previous);
            self.set_debug_doing_this_layout(app, false);
            self.set_debug_mutations_locked(app, false);
        }
        self.set_needs_layout(app, false);
        self.mark_needs_paint(app);
    }

    /// Shared `layout` body. Protocol types store their own constraints.
    pub(crate) fn run_layout(
        self,
        app: &mut App,
        parent_uses_size: bool,
        constraints_tight: bool,
        same_constraints: bool,
        store_constraints: impl FnOnce(&mut App),
    ) {
        debug_assert!(!self.debug_doing_this_resize(app));
        debug_assert!(!self.debug_doing_this_layout(app));
        if cfg!(debug_assertions) {
            self.set_debug_can_parent_use_size(app, Some(parent_uses_size));
        }

        let sized_by_parent = self.sized_by_parent(app);
        self.set_is_relayout_boundary(
            app,
            Some(
                !parent_uses_size
                    || sized_by_parent
                    || constraints_tight
                    || self.parent(app).is_none(),
            ),
        );
        if !self.needs_layout(app) && same_constraints {
            return;
        }
        store_constraints(app);

        if sized_by_parent {
            if cfg!(debug_assertions) {
                self.set_debug_doing_this_resize(app, true);
            }
            (self.vtable.perform_resize)(app, self.id);
            if cfg!(debug_assertions) {
                self.set_debug_doing_this_resize(app, false);
            }
        }

        let previous = if cfg!(debug_assertions) {
            debug_assert!(!self.doing_this_layout_with_callback(app));
            self.set_debug_mutations_locked(app, true);
            self.set_debug_doing_this_layout(app, true);
            let previous = AnyRenderObject::debug_active_layout();
            AnyRenderObject::set_debug_active_layout(Some(self));
            if crate::debug::debug_print_layouts() {
                eprintln!(
                    "Laying out ({}) {self:?}",
                    if sized_by_parent {
                        "with separate resize"
                    } else {
                        "with resize allowed"
                    }
                );
            }
            previous
        } else {
            None
        };
        (self.vtable.perform_layout)(app, self.id);
        if cfg!(debug_assertions) {
            AnyRenderObject::set_debug_active_layout(previous);
            self.set_debug_doing_this_layout(app, false);
            self.set_debug_mutations_locked(app, false);
        }
        self.set_needs_layout(app, false);
        self.mark_needs_paint(app);
    }

    // ---- paint ------------------------------------------------------------------------------

    pub(crate) fn needs_paint(self, app: &App) -> bool {
        self.data(app).needs_paint
    }

    fn set_needs_paint(self, app: &mut App, value: bool) {
        self.data_mut(app).needs_paint = value;
    }

    /// Whether this render object's paint information is dirty.
    ///
    /// Always `false` when debug assertions are disabled.
    pub fn debug_needs_paint(self, app: &App) -> bool {
        if !cfg!(debug_assertions) {
            return false;
        }
        self.needs_paint(app)
    }

    pub(crate) fn needs_composited_layer_update(self, app: &App) -> bool {
        self.data(app).needs_composited_layer_update
    }

    pub(crate) fn set_needs_composited_layer_update(self, app: &mut App, value: bool) {
        self.data_mut(app).needs_composited_layer_update = value;
    }

    /// Whether this render object's layer information is dirty.
    ///
    /// Always `false` when debug assertions are disabled.
    pub fn debug_needs_composited_layer_update(self, app: &App) -> bool {
        if !cfg!(debug_assertions) {
            return false;
        }
        self.needs_composited_layer_update(app)
    }

    pub(crate) fn was_repaint_boundary(self, app: &App) -> bool {
        self.data(app).was_repaint_boundary
    }

    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `renderObject as T`: the typed handle when this object is a `T`, else `None`.
    pub fn downcast<T: RenderObject>(self, app: &App) -> Option<RenderHandle<T>> {
        app.handle::<T>(self.id).map(RenderHandle::from_handle)
    }

    /// Release any resources held by this render object and free its arena slot. Dart's
    /// `dispose` nulls the layer handle and leaves the object to the collector; the arena has no
    /// collector, so the slot is destroyed here after the virtual [`RenderObject::dispose`].
    ///
    /// The object must be detached. Its parent may still hold a handle to it: the widget layer
    /// unmounts bottom-up and disposes the parent next, as Dart does.
    pub fn dispose(self, app: &mut App) {
        debug_assert!(!self.attached(app));
        (self.vtable.dispose)(app, self.id);
        app.destroy(self.id);
    }

    /// Whether this render object repaints separately from its parent.
    pub fn is_repaint_boundary(self, app: &App) -> bool {
        (self.vtable.is_repaint_boundary)(app, self.id)
    }

    /// An estimate of the bounds within which this render object will paint.
    pub fn paint_bounds(self, app: &App) -> Rect {
        (self.vtable.paint_bounds)(app, self.id)
    }

    /// See [`RenderObject::apply_paint_transform`].
    pub fn apply_paint_transform(self, app: &App, child: AnyRenderObject, transform: &mut Matrix4) {
        (self.vtable.apply_paint_transform)(app, self.id, child, transform)
    }

    /// Applies the paint transform from this [`AnyRenderObject`] to the `target`
    /// [`AnyRenderObject`].
    ///
    /// Returns a matrix that maps the local paint coordinate system to the coordinate system
    /// of `target`, or a `Matrix4::zero()` if the paint transform can not be computed.
    ///
    /// This method throws an exception when the `target` is not in the same render tree as
    /// this render object.
    ///
    /// The `target` argument defaults to the root of the render tree (`None`).
    pub fn get_transform_to(self, app: &App, target: Option<AnyRenderObject>) -> Matrix4 {
        debug_assert!(self.attached(app));
        // The paths from to fromRenderObject and toRenderObject's common ancestor.
        // Each list's length is greater than 1 if not null.
        //
        // [this, ...., commonAncestorRenderObject], or null if `this` is the common
        // ancestor.
        let mut from_path: Option<Vec<AnyRenderObject>> = None;
        // [target, ...., commonAncestorRenderObject], or null if `target` is the
        // common ancestor.
        let mut to_path: Option<Vec<AnyRenderObject>> = None;

        let mut from = self;
        let mut to = target.unwrap_or_else(|| {
            self.owner(app)
                .and_then(|owner| owner.root_node(app))
                .expect("an attached render object has a root")
        });

        while from != to {
            let from_depth = from.depth(app);
            let to_depth = to.depth(app);

            if from_depth >= to_depth {
                let from_parent = from.parent(app).unwrap_or_else(|| {
                    panic!("{target:?} and {self:?} are not in the same render tree.")
                });
                from_path
                    .get_or_insert_with(|| vec![self])
                    .push(from_parent);
                from = from_parent;
            }
            if from_depth <= to_depth {
                let to_parent = to.parent(app).unwrap_or_else(|| {
                    panic!("{target:?} and {self:?} are not in the same render tree.")
                });
                debug_assert!(
                    target.is_some(),
                    "{self:?} has a depth that is less than or equal to the root node"
                );
                to_path
                    .get_or_insert_with(|| vec![target.expect("checked above")])
                    .push(to_parent);
                to = to_parent;
            }
        }

        let mut from_transform: Option<Matrix4> = None;
        if let Some(from_path) = &from_path {
            debug_assert!(from_path.len() > 1);
            let mut transform = Matrix4::IDENTITY;
            let last_index = if target.is_none() {
                from_path.len() - 2
            } else {
                from_path.len() - 1
            };
            for index in (1..=last_index).rev() {
                from_path[index].apply_paint_transform(app, from_path[index - 1], &mut transform);
            }
            from_transform = Some(transform);
        }
        let Some(to_path) = to_path else {
            return from_transform.unwrap_or(Matrix4::IDENTITY);
        };

        debug_assert!(to_path.len() > 1);
        let mut to_transform = Matrix4::IDENTITY;
        for index in (1..to_path.len()).rev() {
            to_path[index].apply_paint_transform(app, to_path[index - 1], &mut to_transform);
        }
        let Some(to_transform) = to_transform.invert() else {
            // If the matrix is singular then `invert()` doesn't do anything.
            return zero_matrix();
        };
        match from_transform {
            Some(mut from_transform) => {
                multiply(&mut from_transform, &to_transform);
                from_transform
            }
            None => to_transform,
        }
    }

    /// See [`RenderObject::mouse_tracker_annotation`]. Also `None` once the object has left
    /// the arena: Dart's tracker keeps a stale annotation object alive, the arena does not.
    pub fn mouse_tracker_annotation(self, app: &App) -> Option<MouseTrackerAnnotation> {
        if !app.contains(self.id) {
            return None;
        }
        (self.vtable.mouse_tracker_annotation)(app, self.id)
    }

    /// See [`RenderObject::interface`]: Dart's `object is RenderAbstractViewport`, as
    /// `interface::<AnyRenderAbstractViewport>()`.
    pub fn interface<I: 'static>(self) -> Option<I> {
        (self.vtable.interface)(self.id, TypeId::of::<I>())?
            .downcast()
            .ok()
            .map(|handle| *handle)
    }

    /// See [`RenderObject::show_on_screen`].
    pub fn show_on_screen(
        self,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        (self.vtable.show_on_screen)(app, self.id, descendant, rect, duration, curve)
    }

    /// See [`RenderObject::update_composited_layer`].
    pub fn update_composited_layer(
        self,
        app: &mut App,
        old_layer: Option<AnyOffsetLayer>,
    ) -> AnyOffsetLayer {
        (self.vtable.update_composited_layer)(app, self.id, old_layer)
    }

    /// Whether this render object always needs compositing.
    pub fn always_needs_compositing(self, app: &App) -> bool {
        (self.vtable.always_needs_compositing)(app, self.id)
    }

    /// The compositing layer that this render object uses to repaint.
    ///
    /// If this render object is not a repaint boundary, it is the responsibility
    /// of the [`RenderObject::paint`] method to populate this field. If
    /// [`needs_compositing`](Self::needs_compositing) is true, this field may be populated with
    /// the root-most layer used by the render object implementation. When repainting, instead of
    /// creating a new layer the render object may update the layer stored in this field for
    /// better performance. It is also OK to leave this field as `None` and create a new layer
    /// on every repaint, but without the performance benefit. If [`needs_compositing`](Self::needs_compositing)
    /// is false, this field must be set to `None` either by never populating this field, or by
    /// setting it to `None` when the value of [`needs_compositing`](Self::needs_compositing)
    /// changes from true to false.
    ///
    /// If this render object is a repaint boundary, the framework automatically
    /// creates an [`OffsetLayer`] and populates this field prior to calling the
    /// [`RenderObject::paint`] method. The paint method must not replace the value of this
    /// field.
    pub fn layer(self, app: &App) -> Option<AnyContainerLayer> {
        debug_assert!(
            !self.is_repaint_boundary(app)
                || self.data(app).layer.layer().is_none()
                || self
                    .data(app)
                    .layer
                    .layer()
                    .is_some_and(|layer| layer.as_offset_layer().is_some())
        );
        self.data(app).layer.layer()
    }

    /// Dart's `layer = newLayer`. The setter asserts this is not a repaint boundary: the
    /// framework creates and assigns an [`OffsetLayer`] to a repaint boundary automatically.
    pub fn set_layer(self, app: &mut App, new_layer: Option<AnyContainerLayer>) {
        debug_assert!(
            !self.is_repaint_boundary(app),
            "Attempted to set a layer to a repaint boundary render object.\n\
             The framework creates and assigns an OffsetLayer to a repaint \
             boundary automatically."
        );
        LayerHandle::set_layer(app, |app| &mut self.data_mut(app).layer, new_layer);
    }

    /// Dart's `layer as T?`.
    pub fn layer_as<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        self.layer(app)
            .and_then(|layer| app.handle::<T>(layer.id()))
    }

    /// In debug mode, the compositing layer that this render object uses to repaint.
    ///
    /// This getter is intended for debugging purposes only. In release builds, it
    /// always returns `None`. In debug builds, it returns the layer even if the layer
    /// is dirty.
    ///
    /// For production code, consider [`layer`](Self::layer).
    pub fn debug_layer(self, app: &App) -> Option<AnyContainerLayer> {
        if !cfg!(debug_assertions) {
            return None;
        }
        self.data(app).layer.layer()
    }

    pub(crate) fn needs_compositing_bits_update(self, app: &App) -> bool {
        self.data(app).needs_compositing_bits_update
    }

    fn set_needs_compositing_bits_update(self, app: &mut App, value: bool) {
        self.data_mut(app).needs_compositing_bits_update = value;
    }

    /// Mark the compositing state for this render object as dirty.
    ///
    /// This is called to indicate that the value for [`needs_compositing`](Self::needs_compositing)
    /// needs to be recomputed during the next [`PipelineOwner::flush_compositing_bits`] engine
    /// phase.
    ///
    /// When the subtree is mutated, we need to recompute our
    /// [`needs_compositing`](Self::needs_compositing) bit, and some of our ancestors need to do
    /// the same (in case ours changed in a way that will change theirs). To this end,
    /// [`adopt_child`](Self::adopt_child) and [`drop_child`](Self::drop_child) call this method,
    /// and, as necessary, this method calls the parent's, etc, walking up the tree to mark all
    /// the nodes that need updating.
    ///
    /// This method does not schedule a rendering frame, because since it cannot be the case
    /// that _only_ the compositing bits changed, something else will have scheduled a frame for
    /// us.
    pub fn mark_needs_compositing_bits_update(self, app: &mut App) {
        debug_assert!(
            !app.is_disposed(self.id),
            "mark_needs_compositing_bits_update on a disposed render object"
        );
        if self.needs_compositing_bits_update(app) {
            return;
        }
        self.set_needs_compositing_bits_update(app, true);
        if let Some(parent) = self.parent(app) {
            if parent.needs_compositing_bits_update(app) {
                return;
            }
            if (!self.was_repaint_boundary(app) || !self.is_repaint_boundary(app))
                && !parent.is_repaint_boundary(app)
            {
                parent.mark_needs_compositing_bits_update(app);
                return;
            }
        }
        // parent is fine (or there isn't one), but we are dirty
        if let Some(owner) = self.owner(app) {
            owner.add_node_needing_compositing_bits_update(app, self);
        }
    }

    /// Whether we or one of our descendants has a compositing layer.
    ///
    /// If this node needs compositing as indicated by this bit, then all ancestor
    /// nodes will also need compositing.
    ///
    /// Only legal to call after [`PipelineOwner::flush_layout`] and
    /// [`PipelineOwner::flush_compositing_bits`] have been called.
    pub fn needs_compositing(self, app: &App) -> bool {
        debug_assert!(
            !self.needs_compositing_bits_update(app),
            "make sure we don't use this bit when it is dirty"
        );
        self.data(app).needs_compositing
    }

    /// Flutter's `_updateCompositingBits`.
    pub(crate) fn update_compositing_bits(self, app: &mut App) {
        if !self.needs_compositing_bits_update(app) {
            return;
        }
        let old_needs_compositing = self.data(app).needs_compositing;
        self.data_mut(app).needs_compositing = false;
        let mut children = Vec::new();
        self.visit_children(app, &mut |child| children.push(child));
        for child in children {
            child.update_compositing_bits(app);
            if child.needs_compositing(app) {
                self.data_mut(app).needs_compositing = true;
            }
        }
        if self.is_repaint_boundary(app) || self.always_needs_compositing(app) {
            self.data_mut(app).needs_compositing = true;
        }
        // If a node was previously a repaint boundary, but no longer is one, then
        // regardless of its compositing state we need to find a new parent to
        // paint from. To do this, we mark it clean again so that the traversal
        // in markNeedsPaint is not short-circuited. It is removed from _nodesNeedingPaint
        // so that we do not attempt to paint from it after locating a parent.
        if !self.is_repaint_boundary(app) && self.was_repaint_boundary(app) {
            self.set_needs_paint(app, false);
            self.set_needs_composited_layer_update(app, false);
            if let Some(owner) = self.owner(app) {
                owner.remove_node_needing_paint(app, self);
            }
            self.set_needs_compositing_bits_update(app, false);
            self.mark_needs_paint(app);
        } else if old_needs_compositing != self.data(app).needs_compositing {
            self.set_needs_compositing_bits_update(app, false);
            self.mark_needs_paint(app);
        } else {
            self.set_needs_compositing_bits_update(app, false);
        }
    }

    /// Mark this render object as having changed its visual appearance.
    ///
    /// Rather than eagerly updating this render object's display list in response to writes,
    /// we instead mark the render object as needing to paint, which schedules a visual update.
    /// As part of the visual update, the rendering pipeline will give this render object an
    /// opportunity to update its display list.
    pub fn mark_needs_paint(self, app: &mut App) {
        debug_assert!(
            !app.is_disposed(self.id),
            "mark_needs_paint on a disposed render object"
        );
        debug_assert!(
            self.owner(app)
                .is_none_or(|owner| !owner.debug_doing_paint(app))
        );
        if self.needs_paint(app) {
            return;
        }
        self.set_needs_paint(app, true);
        if self.is_repaint_boundary(app) && self.was_repaint_boundary(app) {
            if crate::debug::debug_print_mark_needs_paint_stacks() {
                eprintln!("markNeedsPaint() called for {self:?}");
            }
            debug_assert!(
                self.layer(app)
                    .is_some_and(|layer| layer.as_offset_layer().is_some())
            );
            if let Some(owner) = self.owner(app) {
                owner.add_node_needing_paint(app, self);
                owner.request_visual_update(app);
            }
        } else if let Some(parent) = self.parent(app) {
            parent.mark_needs_paint(app);
        } else {
            if crate::debug::debug_print_mark_needs_paint_stacks() {
                eprintln!("markNeedsPaint() called for {self:?} (root of render tree)");
            }
            if let Some(owner) = self.owner(app) {
                owner.request_visual_update(app);
            }
        }
    }

    /// Mark this render object as having changed a property on its composited
    /// layer.
    ///
    /// Render objects that have a composited layer have [`is_repaint_boundary`](Self::is_repaint_boundary)
    /// equal to true may update the properties of that composited layer without repainting
    /// their children. If this render object is a repaint boundary but does
    /// not yet have a composited layer created for it, this method will instead
    /// mark the nearest repaint boundary parent as needing to be painted.
    ///
    /// If this method is called on a render object that is not a repaint boundary
    /// or is a repaint boundary but hasn't been composited yet, it is equivalent
    /// to calling [`mark_needs_paint`](Self::mark_needs_paint).
    pub fn mark_needs_composited_layer_update(self, app: &mut App) {
        debug_assert!(
            !app.is_disposed(self.id),
            "mark_needs_composited_layer_update on a disposed render object"
        );
        debug_assert!(
            self.owner(app)
                .is_none_or(|owner| !owner.debug_doing_paint(app))
        );
        if self.needs_composited_layer_update(app) || self.needs_paint(app) {
            return;
        }
        self.set_needs_composited_layer_update(app, true);
        if self.is_repaint_boundary(app) && self.was_repaint_boundary(app) {
            debug_assert!(self.layer(app).is_some());
            if let Some(owner) = self.owner(app) {
                owner.add_node_needing_paint(app, self);
                owner.request_visual_update(app);
            }
        } else {
            self.mark_needs_paint(app);
        }
    }

    /// Flutter's `_skippedPaintingOnLayer`: this boundary's layer is detached, so the first
    /// attached ancestor boundary will repaint it when it repaints.
    pub(crate) fn skipped_painting_on_layer(self, app: &mut App) {
        debug_assert!(self.attached(app));
        debug_assert!(self.is_repaint_boundary(app));
        debug_assert!(self.needs_paint(app) || self.needs_composited_layer_update(app));
        debug_assert!(self.layer(app).is_some());
        debug_assert!(
            self.layer(app)
                .is_some_and(|layer| !layer.as_layer().attached(app))
        );
        let mut node = self.parent(app);
        while let Some(current) = node {
            if current.is_repaint_boundary(app) {
                match current.layer(app) {
                    None => break,
                    Some(layer) if layer.as_layer().attached(app) => break,
                    Some(_) => current.set_needs_paint(app, true),
                }
            }
            node = current.parent(app);
        }
    }

    /// Bootstrap the rendering pipeline by scheduling the very first paint.
    ///
    /// Requires that this render object is attached, is the root of the render
    /// tree, and has a composited layer.
    pub fn schedule_initial_paint(self, app: &mut App, root_layer: AnyContainerLayer) {
        debug_assert!(root_layer.as_layer().attached(app));
        debug_assert!(self.attached(app));
        debug_assert!(self.parent(app).is_none());
        debug_assert!(!self.owner(app).expect("attached").debug_doing_paint(app));
        debug_assert!(self.is_repaint_boundary(app));
        debug_assert!(self.layer(app).is_none());
        LayerHandle::set_layer(app, |app| &mut self.data_mut(app).layer, Some(root_layer));
        debug_assert!(self.needs_paint(app));
        let owner = self.owner(app).expect("attached");
        owner.add_node_needing_paint(app, self);
    }

    /// Replace the layer. This is only valid for the root of a render
    /// object subtree (whatever object [`schedule_initial_paint`](Self::schedule_initial_paint)
    /// was called on).
    ///
    /// This might be called if, e.g., the device pixel ratio changed.
    pub fn replace_root_layer(self, app: &mut App, root_layer: AnyOffsetLayer) {
        debug_assert!(
            !app.is_disposed(self.id),
            "replace_root_layer on a disposed render object"
        );
        debug_assert!(root_layer.as_layer().attached(app));
        debug_assert!(self.attached(app));
        debug_assert!(self.parent(app).is_none());
        debug_assert!(!self.owner(app).expect("attached").debug_doing_paint(app));
        debug_assert!(self.is_repaint_boundary(app));
        debug_assert!(
            self.layer(app).is_some(),
            "use schedule_initial_paint the first time"
        );
        self.layer(app).expect("checked").as_layer().detach(app);
        LayerHandle::set_layer(
            app,
            |app| &mut self.data_mut(app).layer,
            Some(root_layer.as_container_layer()),
        );
        self.mark_needs_paint(app);
    }

    /// Flutter's `_paintWithContext`: the framework wrapper around the `paint` virtual.
    pub(crate) fn paint_with_context(
        self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        debug_assert!(
            !self.data(app).debug_doing_this_paint,
            "Tried to paint a RenderObject reentrantly: {self:?}"
        );
        if self.needs_layout(app) {
            return;
        }
        debug_assert!(
            !self.needs_compositing_bits_update(app),
            "Tried to paint a RenderObject before its compositing bits were updated.\n\
             The following RenderObject was marked as having dirty compositing bits \
             at the time that it was painted: {self:?}\n\
             A RenderObject that still has dirty compositing bits cannot be painted \
             because this indicates that the tree has not yet been properly configured \
             for creating the layer tree.\n\
             This usually indicates an error in the Flutter framework itself."
        );
        let debug_last_active_paint = if cfg!(debug_assertions) {
            self.data_mut(app).debug_doing_this_paint = true;
            let previous = DEBUG_ACTIVE_PAINT.replace(Some(self));
            debug_assert!(!self.is_repaint_boundary(app) || self.layer(app).is_some());
            previous
        } else {
            None
        };
        self.set_needs_paint(app, false);
        self.set_needs_composited_layer_update(app, false);
        let is_repaint_boundary = self.is_repaint_boundary(app);
        self.data_mut(app).was_repaint_boundary = is_repaint_boundary;
        (self.vtable.paint)(app, self.id, context, offset);
        debug_assert!(!self.needs_layout(app));
        debug_assert!(!self.needs_paint(app));
        if cfg!(debug_assertions) {
            DEBUG_ACTIVE_PAINT.set(debug_last_active_paint);
            self.data_mut(app).debug_doing_this_paint = false;
        }
    }
}

/// Flutter's `RenderObject` bodies that an override calls through `super`.
///
/// Blanket-implemented for every [`RenderObject`], and repeats the default bodies of the
/// virtuals above: a leaf's own override shadows the default it would otherwise call.
pub trait RenderObjectBase: RenderObject {
    /// Flutter's `RenderObject.dispose` body: releases the layer handle. An override calls it
    /// last, where Dart writes `super.dispose()`.
    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut self.render_object_data_mut(app).layer, None);
    }

    /// Flutter's `RenderObject.markNeedsLayout` body; an override calls it where Dart writes
    /// `super.markNeedsLayout()`.
    fn mark_needs_layout(self: RenderHandle<Self>, app: &mut App) {
        let this = self.as_render_object(app);
        if this.needs_layout(app) {
            return;
        }
        this.set_needs_layout(app, true);
        if let Some(owner) = this.owner(app)
            && this.is_relayout_boundary(app).unwrap_or(false)
        {
            if crate::debug::debug_print_mark_needs_layout_stacks() {
                eprintln!("markNeedsLayout() called for {this:?}");
            }
            owner.add_node_needing_layout(app, this);
            owner.request_visual_update(app);
        } else if this.parent(app).is_some() {
            this.mark_parent_needs_layout(app);
        }
    }

    /// Flutter's `RenderObject.showOnScreen`.
    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        let this = self.as_render_object(app);
        if let Some(parent) = this.parent(app) {
            parent.show_on_screen(app, Some(descendant.unwrap_or(this)), rect, duration, curve);
        }
    }
}

impl<T: RenderObject> RenderObjectBase for T {}

/// vector_math's `Matrix4.zero()`.
pub(crate) fn zero_matrix() -> Matrix4 {
    Matrix4::from_flutter_array(&[0.0; 16])
}

/// vector_math's `transform.multiply(other)`: `transform = transform * other`, so `other`
/// applies first to a point.
pub(crate) fn multiply(transform: &mut Matrix4, other: &Matrix4) {
    *transform = transform.then(other);
}

/// vector_math's `transform.translate(dx, dy)`.
pub(crate) fn translate(transform: &mut Matrix4, dx: f64, dy: f64) {
    multiply(transform, &Matrix4::translation(dx as f32, dy as f32));
}

/// vector_math's `Matrix4.perspectiveTransform(Vector3)`: transforms the point and divides by
/// the resulting `w`.
pub(crate) fn perspective_transform(transform: &Matrix4, point: [f64; 3]) -> [f64; 3] {
    let m = transform.to_flutter_array().map(f64::from);
    let [x, y, z] = point;
    let out = |row: usize| m[row] * x + m[4 + row] * y + m[8 + row] * z + m[12 + row];
    let w = out(3);
    [out(0) / w, out(1) / w, out(2) / w]
}

/// The bag of [`RenderObjectWithLayoutCallbackMixin`].
#[derive(Debug)]
pub struct RenderObjectWithLayoutCallbackData {
    // The initial value of this flag must be set to true to prevent the layout
    // callback from being scheduled when the subtree has never been laid out (in
    // which case the `constraints` or any other layout information is unknown).
    needs_rebuild: bool,
}

impl Default for RenderObjectWithLayoutCallbackData {
    fn default() -> RenderObjectWithLayoutCallbackData {
        RenderObjectWithLayoutCallbackData {
            needs_rebuild: true,
        }
    }
}

/// A mixin for `RenderObject`s that run a layout callback (typically one that builds a
/// widget subtree) during `perform_layout`, as `LayoutBuilder`'s render object does.
///
/// Implementers keep the bag under a field named `layout_callback`, implement
/// [`layout_callback`](Self::layout_callback), and call
/// [`run_layout_callback`](Self::run_layout_callback) in their `perform_layout`. The mixin
/// is on the box protocol here because scheduling needs the tree methods.
pub trait RenderObjectWithLayoutCallbackMixin: crate::box_::RenderBox {
    /// The mixin's bag.
    fn layout_callback_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderObjectWithLayoutCallbackData;

    /// See [`layout_callback_data`](Self::layout_callback_data).
    fn layout_callback_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithLayoutCallbackData;

    /// The layout callback to run in `perform_layout`, through
    /// [`run_layout_callback`](Self::run_layout_callback), which invokes it with
    /// [`AnyRenderObject::invoke_layout_callback`].
    fn layout_callback(self: RenderHandle<Self>, app: &mut App);

    /// Invokes [`layout_callback`](Self::layout_callback) with
    /// [`AnyRenderObject::invoke_layout_callback`].
    ///
    /// Must be called in `perform_layout`, and only after the callback was scheduled.
    fn run_layout_callback(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(self.as_object().debug_doing_this_layout(app));
        self.as_object()
            .invoke_layout_callback(app, |app| Self::layout_callback(self, app));
        self.layout_callback_data_mut(app).needs_rebuild = false;
    }

    /// Schedules the layout callback to be run in the next layout pass, marking this object
    /// for layout.
    fn schedule_layout_callback(self: RenderHandle<Self>, app: &mut App) {
        if self.layout_callback_data(app).needs_rebuild {
            debug_assert!(self.as_object().needs_layout(app));
            return;
        }
        self.layout_callback_data_mut(app).needs_rebuild = true;
        // This ensures that the layout callback will be run even if an ancestor
        // chooses to not lay out this subtree (for example, obstructed OverlayEntries
        // with `maintainState` set to true), to maintain the widget tree integrity
        // (making sure global keys are unique, for example).
        if let Some(owner) = self.as_object().owner(app) {
            owner.add_node_needing_layout(app, self.as_object());
        }
        // In an active tree, markNeedsLayout is needed to inform the layout boundary
        // that its child size may change.
        self.as_object().mark_needs_layout(app);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_foundation::{App, AppCell, Listener};

    use super::*;
    use crate::box_::{BoxConstraints, RenderBox, RenderBoxData};
    use crate::pipeline_owner::PipelineOwner;

    struct TestLeaf {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        layouts: Rc<Cell<u32>>,
    }

    impl RenderObject for TestLeaf {
        render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let layouts = self.get(app).layouts.clone();
            layouts.set(layouts.get() + 1);
            let size = self.constraints(app).smallest();
            self.set_size(app, size);
        }
    }

    impl RenderBox for TestLeaf {
        crate::render_box_accessors!();
    }

    struct TestParent {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        child: Option<crate::AnyRenderBox>,
        layouts: Rc<Cell<u32>>,
        parent_uses_size: bool,
        lay_out_child: bool,
    }

    impl RenderObject for TestParent {
        render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let layouts = self.get(app).layouts.clone();
            layouts.set(layouts.get() + 1);
            let child = self.get(app).child;
            let parent_uses_size = self.get(app).parent_uses_size;
            let lay_out_child = self.get(app).lay_out_child;
            let constraints = self.constraints(app);
            if let Some(child) = child
                && lay_out_child
            {
                child.layout(app, constraints, parent_uses_size);
                let _again = self.get(app).layouts.get();
                if parent_uses_size {
                    self.set_size(app, child.size(app));
                    return;
                }
            }
            self.set_size(app, constraints.smallest());
        }

        fn visit_children(
            self: RenderHandle<Self>,
            app: &App,
            visitor: &mut dyn FnMut(AnyRenderObject),
        ) {
            if let Some(child) = self.get(app).child {
                visitor(child.as_object());
            }
        }
    }

    impl RenderBox for TestParent {
        crate::render_box_accessors!();
    }

    fn tight() -> BoxConstraints {
        BoxConstraints::tight_for_finite(100.0, 100.0)
    }

    fn loose() -> BoxConstraints {
        BoxConstraints::new()
    }

    fn leaf(layouts: Rc<Cell<u32>>) -> TestLeaf {
        TestLeaf {
            render_object: RenderObjectData::new(),
            render_box: RenderBoxData::new(),
            layouts,
        }
    }

    fn parent(
        child: Option<crate::AnyRenderBox>,
        layouts: Rc<Cell<u32>>,
        parent_uses_size: bool,
        lay_out_child: bool,
    ) -> TestParent {
        TestParent {
            render_object: RenderObjectData::new(),
            render_box: RenderBoxData::new(),
            child,
            layouts,
            parent_uses_size,
            lay_out_child,
        }
    }

    /// `object_test.dart`: `nodesNeedingLayout updated with layout changes`.
    #[test]
    fn nodes_needing_layout_updated_with_layout_changes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = PipelineOwner::new(&mut app, None);
        let node = RenderHandle::new_box(&mut app, leaf(Rc::new(Cell::new(0))));
        node.as_object().attach(&mut app, owner);
        assert!(owner.nodes_needing_layout(&app).is_empty());

        node.layout(&mut app, tight(), false);
        node.as_object().mark_needs_layout(&mut app);
        assert!(owner.nodes_needing_layout(&app).contains(&node.as_object()));

        owner.flush_layout(&mut app);
        assert!(owner.nodes_needing_layout(&app).is_empty());
    }

    #[test]
    fn clean_layout_skips_perform_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let layouts = Rc::new(Cell::new(0));
        let node = RenderHandle::new_box(&mut app, leaf(Rc::clone(&layouts)));
        node.layout(&mut app, tight(), false);
        assert_eq!(layouts.get(), 1);
        node.layout(&mut app, tight(), false);
        assert_eq!(layouts.get(), 1);
        node.as_object().mark_needs_layout(&mut app);
        node.layout(&mut app, tight(), false);
        assert_eq!(layouts.get(), 2);
    }

    #[test]
    fn parent_uses_size_marks_parent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = PipelineOwner::new(&mut app, None);
        let child = RenderHandle::new_box(&mut app, leaf(Rc::new(Cell::new(0))));
        let parent = RenderHandle::new_box(
            &mut app,
            parent(Some(child.as_box()), Rc::new(Cell::new(0)), true, true),
        );
        parent.adopt_child(&mut app, child.as_object());
        parent.as_object().attach(&mut app, owner);
        parent.layout(&mut app, loose(), false);
        assert!(!parent.debug_needs_layout(&app));
        assert!(!child.debug_needs_layout(&app));

        child.as_object().mark_needs_layout(&mut app);
        assert!(parent.debug_needs_layout(&app));
        assert!(
            owner
                .nodes_needing_layout(&app)
                .contains(&parent.as_object())
        );
    }

    #[test]
    fn adopt_drop_attach_detach() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = PipelineOwner::new(&mut app, None);
        let child = RenderHandle::new_box(&mut app, leaf(Rc::new(Cell::new(0))));
        let parent = RenderHandle::new_box(
            &mut app,
            parent(Some(child.as_box()), Rc::new(Cell::new(0)), false, true),
        );
        parent.adopt_child(&mut app, child.as_object());
        assert_eq!(child.parent(&app), Some(parent.as_object()));
        assert_eq!(
            child.as_object().depth(&app),
            parent.as_object().depth(&app) + 1
        );
        assert!(!child.attached(&app));

        parent.as_object().attach(&mut app, owner);
        assert!(parent.attached(&app));
        assert!(child.attached(&app));
        assert_eq!(child.owner(&app), Some(owner));

        parent.drop_child(&mut app, child.as_object());
        assert!(child.parent(&app).is_none());
        assert!(!child.attached(&app));
        assert!(parent.attached(&app));
    }

    #[test]
    fn parent_reenters_itself_after_child_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let layouts = Rc::new(Cell::new(0));
        let child = RenderHandle::new_box(&mut app, leaf(Rc::new(Cell::new(0))));
        let parent = RenderHandle::new_box(
            &mut app,
            parent(Some(child.as_box()), Rc::clone(&layouts), true, true),
        );
        parent.adopt_child(&mut app, child.as_object());
        parent.layout(&mut app, loose(), false);
        assert_eq!(layouts.get(), 1);
        assert_eq!(parent.size(&app), child.size(&app));
    }

    #[test]
    fn flush_layout_orders_by_depth() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = PipelineOwner::new(&mut app, None);
        let order = Rc::new(std::cell::RefCell::new(Vec::new()));
        let child_order = Rc::clone(&order);
        let parent_order = Rc::clone(&order);

        struct OrderLeaf {
            render_object: RenderObjectData,
            render_box: RenderBoxData,
            name: &'static str,
            order: Rc<std::cell::RefCell<Vec<&'static str>>>,
        }
        impl RenderObject for OrderLeaf {
            render_object_accessors!();

            fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
                let name = self.get(app).name;
                self.get(app).order.borrow_mut().push(name);
                let size = self.constraints(app).smallest();
                self.set_size(app, size);
            }
        }
        impl RenderBox for OrderLeaf {
            crate::render_box_accessors!();
        }

        struct OrderParent {
            render_object: RenderObjectData,
            render_box: RenderBoxData,
            name: &'static str,
            child: Option<crate::AnyRenderBox>,
            order: Rc<std::cell::RefCell<Vec<&'static str>>>,
        }
        impl RenderObject for OrderParent {
            render_object_accessors!();

            fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
                let name = self.get(app).name;
                self.get(app).order.borrow_mut().push(name);
                let size = self.constraints(app).smallest();
                self.set_size(app, size);
            }
            fn visit_children(
                self: RenderHandle<Self>,
                app: &App,
                visitor: &mut dyn FnMut(AnyRenderObject),
            ) {
                if let Some(child) = self.get(app).child {
                    visitor(child.as_object());
                }
            }
        }
        impl RenderBox for OrderParent {
            crate::render_box_accessors!();
        }

        let child = RenderHandle::new_box(
            &mut app,
            OrderLeaf {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                name: "child",
                order: child_order,
            },
        );
        let parent = RenderHandle::new_box(
            &mut app,
            OrderParent {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                name: "parent",
                child: Some(child.as_box()),
                order: parent_order,
            },
        );
        parent.adopt_child(&mut app, child.as_object());
        parent.as_object().attach(&mut app, owner);
        parent.layout(&mut app, tight(), false);
        child.layout(&mut app, tight(), false);
        parent.as_object().mark_needs_layout(&mut app);
        child.as_object().mark_needs_layout(&mut app);
        order.borrow_mut().clear();
        owner.flush_layout(&mut app);
        assert_eq!(*order.borrow(), ["parent", "child"]);
    }

    #[test]
    fn mark_needs_layout_requests_visual_update() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let updates = Rc::new(Cell::new(0));
        let updates_cb = Rc::clone(&updates);
        let owner = PipelineOwner::new(
            &mut app,
            Some(Listener::new(move |_| {
                updates_cb.set(updates_cb.get() + 1);
            })),
        );
        let node = RenderHandle::new_box(&mut app, leaf(Rc::new(Cell::new(0))));
        owner.set_root_node(&mut app, Some(node.as_object()));
        node.layout(&mut app, tight(), false);
        assert_eq!(updates.get(), 0);
        node.as_object().mark_needs_layout(&mut app);
        assert_eq!(updates.get(), 1);
    }
}

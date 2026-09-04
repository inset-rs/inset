//! Flutter counterpart: `rendering/object.dart` (`Constraints`, `ParentData`,
//! `RenderObject` tree / layout / paint marks). `PaintingContext` is in
//! `painting_context.rs`, `PipelineOwner` in `pipeline_owner.rs`.
//!
//! Semantics wait. A concrete node is [`RenderHandle<T>`] over the authored
//! struct. Tree edges are [`AnyRenderObject`]. Authored methods take
//! `self: RenderHandle<Self>`.

use std::any::Any;
use std::cell::Cell;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Receiver;

use reveal_embedder::{Offset, Rect};
use reveal_foundation::{App, Handle, HandleId};

use crate::layer::{BoundaryLayer, CompositedLayer};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

thread_local! {
    static DEBUG_ACTIVE_LAYOUT: Cell<Option<AnyRenderObject>> = const { Cell::new(None) };
    static DEBUG_ACTIVE_PAINT: Cell<Option<AnyRenderObject>> = const { Cell::new(None) };
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
    pub(crate) parent: Option<AnyRenderObject>,
    pub(crate) owner: Option<PipelineOwner>,
    pub(crate) depth: i32,
    pub(crate) parent_data: Option<Box<dyn ParentData>>,
    pub(crate) needs_layout: bool,
    pub(crate) needs_paint: bool,
    pub(crate) needs_composited_layer_update: bool,
    pub(crate) was_repaint_boundary: bool,
    /// Flutter's `_layerHandle`: the repaint boundary's layer, `None` until first painted.
    pub(crate) layer: Option<BoundaryLayer>,
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
            parent: None,
            owner: None,
            depth: 0,
            parent_data: None,
            needs_layout: true,
            needs_paint: true,
            needs_composited_layer_update: false,
            was_repaint_boundary: false,
            layer: None,
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

    /// Do the work of computing the layout for this render object.
    ///
    /// Do not call this function directly: call `layout` on the protocol handle instead.
    /// Read the constraints with `constraints` on the protocol trait.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App);

    /// Updates the render object's size using only the constraints.
    ///
    /// Called by `layout` only when [`sized_by_parent`](Self::sized_by_parent) is true.
    /// Override this instead of relying on `computeDryLayout`, which is not in this slice.
    fn perform_resize(self: RenderHandle<Self>, _app: &mut App) {
        let _ = self;
        panic!(
            "RenderObject::perform_resize was called; override it when sized_by_parent is true \
             (computeDryLayout is not in this slice)"
        )
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
    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: PipelineOwner) {
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

    /// Whether this render object repaints separately from its parent.
    ///
    /// A repaint boundary composites its own recording through
    /// [`update_composited_layer`](Self::update_composited_layer): repainting it does not repaint
    /// the parent, and repainting the parent reuses its recording. The value may change only when
    /// the parent is also marked for paint (see `RenderOpacity`).
    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        let _ = self;
        false
    }

    /// Update the composited layer owned by this render object.
    ///
    /// Called on repaint boundaries only: when the boundary repaints, and after
    /// [`AnyRenderObject::mark_needs_composited_layer_update`] without a repaint. Return
    /// `old_layer` with its properties updated, or a new layer when `old_layer` is `None`. The
    /// offset belongs to the parent; do not change it.
    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<CompositedLayer>,
    ) -> CompositedLayer {
        debug_assert!(self.is_repaint_boundary(app));
        old_layer.unwrap_or_default()
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
    pub(crate) fn handle(self) -> Handle<T> {
        self.0
    }

    pub(crate) fn from_handle(handle: Handle<T>) -> RenderHandle<T> {
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

/// The vtable of an erased [`AnyRenderObject`]: one `&'static` table per type, built from the
/// trait impl by the protocol. Copied out of the edge before a virtual call, so the slot is not
/// borrowed across it.
pub(crate) struct RenderObjectVTable {
    pub object_data: fn(&App, HandleId) -> &RenderObjectData,
    pub object_data_mut: fn(&mut App, HandleId) -> &mut RenderObjectData,
    pub visit_children: fn(&App, HandleId, &mut dyn FnMut(AnyRenderObject)),
    pub did_attach: fn(&mut App, HandleId, PipelineOwner),
    pub did_detach: fn(&mut App, HandleId),
    pub redepth_children: fn(&mut App, HandleId),
    pub setup_parent_data: fn(&mut App, HandleId, AnyRenderObject),
    pub sized_by_parent: fn(&App, HandleId) -> bool,
    pub perform_layout: fn(&mut App, HandleId),
    pub perform_resize: fn(&mut App, HandleId),
    pub paint_bounds: fn(&App, HandleId) -> Rect,
    pub is_repaint_boundary: fn(&App, HandleId) -> bool,
    pub update_composited_layer: fn(&mut App, HandleId, Option<CompositedLayer>) -> CompositedLayer,
    pub paint: fn(&mut App, HandleId, &mut PaintingContext, Offset),
    /// The protocol table this object table is nested in. Exactly one is `Some`.
    pub as_box: Option<fn() -> &'static crate::box_::RenderBoxVTable>,
    pub as_sliver: Option<fn() -> &'static crate::sliver::RenderSliverVTable>,
}

impl RenderObjectVTable {
    /// `setup_parent_data` is passed in because the box protocol has its own default;
    /// `paint_bounds` because each protocol defines it.
    pub(crate) const fn of<T: RenderObject>(
        setup_parent_data: fn(&mut App, HandleId, AnyRenderObject),
        paint_bounds: fn(&App, HandleId) -> Rect,
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
            perform_resize: |app, id| T::perform_resize(resolve(id), app),
            paint_bounds,
            is_repaint_boundary: |app, id| T::is_repaint_boundary(resolve(id), app),
            update_composited_layer: |app, id, old_layer| {
                T::update_composited_layer(resolve(id), app, old_layer)
            },
            paint: |app, id, context, offset| T::paint(resolve(id), app, context, offset),
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

    fn data_mut(self, app: &mut App) -> &mut RenderObjectData {
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
    pub fn owner(self, app: &App) -> Option<PipelineOwner> {
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

    fn debug_doing_this_layout(self, app: &App) -> bool {
        self.data(app).debug_doing_this_layout
    }

    fn set_debug_doing_this_layout(self, app: &mut App, value: bool) {
        self.data_mut(app).debug_doing_this_layout = value;
    }

    fn debug_doing_this_resize(self, app: &App) -> bool {
        self.data(app).debug_doing_this_resize
    }

    fn set_debug_doing_this_resize(self, app: &mut App, value: bool) {
        self.data_mut(app).debug_doing_this_resize = value;
    }

    fn set_debug_can_parent_use_size(self, app: &mut App, value: Option<bool>) {
        self.data_mut(app).debug_can_parent_use_size = value;
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
    }

    /// Mark this render object as attached to `owner`.
    ///
    /// Flutter's `RenderObject.attach` body, then the [`RenderObject::did_attach`] hook, which
    /// by default attaches the children.
    pub fn attach(self, app: &mut App, owner: PipelineOwner) {
        debug_assert!(self.owner(app).is_none());
        self.data_mut(app).owner = Some(owner);
        if self.needs_layout(app) && self.is_relayout_boundary(app).is_some() {
            self.set_needs_layout(app, false);
            self.mark_needs_layout(app);
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
    pub fn mark_needs_layout(self, app: &mut App) {
        if self.needs_layout(app) {
            return;
        }
        self.set_needs_layout(app, true);
        if let Some(owner) = self.owner(app)
            && self.is_relayout_boundary(app).unwrap_or(false)
        {
            if crate::debug::debug_print_mark_needs_layout_stacks() {
                eprintln!("markNeedsLayout() called for {self:?}");
            }
            owner.add_node_needing_layout(app, self);
            owner.request_visual_update(app);
        } else if self.parent(app).is_some() {
            self.mark_parent_needs_layout(app);
        }
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

    /// Whether this render object repaints separately from its parent.
    pub fn is_repaint_boundary(self, app: &App) -> bool {
        (self.vtable.is_repaint_boundary)(app, self.id)
    }

    /// An estimate of the bounds within which this render object will paint.
    pub fn paint_bounds(self, app: &App) -> Rect {
        (self.vtable.paint_bounds)(app, self.id)
    }

    /// See [`RenderObject::update_composited_layer`].
    pub fn update_composited_layer(
        self,
        app: &mut App,
        old_layer: Option<CompositedLayer>,
    ) -> CompositedLayer {
        (self.vtable.update_composited_layer)(app, self.id, old_layer)
    }

    pub(crate) fn layer(self, app: &App) -> Option<&BoundaryLayer> {
        self.data(app).layer.as_ref()
    }

    pub(crate) fn layer_mut(self, app: &mut App) -> Option<&mut BoundaryLayer> {
        self.data_mut(app).layer.as_mut()
    }

    pub(crate) fn set_layer(self, app: &mut App, layer: Option<BoundaryLayer>) {
        self.data_mut(app).layer = layer;
    }

    /// In debug mode, the layer of this repaint boundary. Always `None` when debug assertions
    /// are disabled.
    pub fn debug_layer(self, app: &App) -> Option<&BoundaryLayer> {
        if !cfg!(debug_assertions) {
            return None;
        }
        self.layer(app)
    }

    /// Mark this render object as having changed its visual appearance.
    ///
    /// Rather than eagerly updating this render object's display list in response to writes,
    /// we instead mark the render object as needing to paint, which schedules a visual update.
    /// As part of the visual update, the rendering pipeline will give this render object an
    /// opportunity to update its display list.
    pub fn mark_needs_paint(self, app: &mut App) {
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
            debug_assert!(self.layer(app).is_some());
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

    /// Mark this render object as having changed a property on its composited layer.
    ///
    /// Render objects that have a composited layer have their [`layer`](Self::debug_layer)
    /// updated through [`RenderObject::update_composited_layer`] without repainting their
    /// children.
    pub fn mark_needs_composited_layer_update(self, app: &mut App) {
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
        debug_assert!(self.layer(app).is_some_and(|layer| !layer.attached));
        let mut node = self.parent(app);
        while let Some(current) = node {
            if current.is_repaint_boundary(app) {
                match current.layer(app) {
                    None => break,
                    Some(layer) if layer.attached => break,
                    Some(_) => current.set_needs_paint(app, true),
                }
            }
            node = current.parent(app);
        }
    }

    /// Bootstrap the rendering pipeline by scheduling the very first paint.
    ///
    /// Requires that this render object is attached, is the root of the render tree, and has a
    /// composited layer. `root_layer` is attached: it is the layer the host composes.
    pub fn schedule_initial_paint(self, app: &mut App, root_layer: CompositedLayer) {
        debug_assert!(self.attached(app));
        debug_assert!(self.parent(app).is_none());
        debug_assert!(!self.owner(app).expect("attached").debug_doing_paint(app));
        debug_assert!(self.is_repaint_boundary(app));
        debug_assert!(self.layer(app).is_none());
        self.set_layer(app, Some(BoundaryLayer::new(root_layer, true)));
        debug_assert!(self.needs_paint(app));
        let owner = self.owner(app).expect("attached");
        owner.add_node_needing_paint(app, self);
    }

    /// Replace the layer. This is only valid for the root of a render object subtree (whatever
    /// object [`schedule_initial_paint`](Self::schedule_initial_paint) was called on).
    pub fn replace_root_layer(self, app: &mut App, root_layer: CompositedLayer) {
        debug_assert!(self.attached(app));
        debug_assert!(self.parent(app).is_none());
        debug_assert!(!self.owner(app).expect("attached").debug_doing_paint(app));
        debug_assert!(self.is_repaint_boundary(app));
        debug_assert!(
            self.layer(app).is_some(),
            "use schedule_initial_paint the first time"
        );
        self.detach_layer(app);
        self.set_layer(app, Some(BoundaryLayer::new(root_layer, true)));
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_foundation::{App, Listener};

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
        let mut app = App::new();
        let owner = PipelineOwner::new(&mut app, None);
        let node = RenderHandle::new_box(&mut app, leaf(Rc::new(Cell::new(0))));
        node.as_object().attach(&mut app, owner);
        assert!(owner.nodes_needing_layout(&app).is_empty());

        node.layout(&mut app, tight(), false);
        node.mark_needs_layout(&mut app);
        assert!(owner.nodes_needing_layout(&app).contains(&node.as_object()));

        owner.flush_layout(&mut app);
        assert!(owner.nodes_needing_layout(&app).is_empty());
    }

    #[test]
    fn clean_layout_skips_perform_layout() {
        let mut app = App::new();
        let layouts = Rc::new(Cell::new(0));
        let node = RenderHandle::new_box(&mut app, leaf(Rc::clone(&layouts)));
        node.layout(&mut app, tight(), false);
        assert_eq!(layouts.get(), 1);
        node.layout(&mut app, tight(), false);
        assert_eq!(layouts.get(), 1);
        node.mark_needs_layout(&mut app);
        node.layout(&mut app, tight(), false);
        assert_eq!(layouts.get(), 2);
    }

    #[test]
    fn parent_uses_size_marks_parent() {
        let mut app = App::new();
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

        child.mark_needs_layout(&mut app);
        assert!(parent.debug_needs_layout(&app));
        assert!(
            owner
                .nodes_needing_layout(&app)
                .contains(&parent.as_object())
        );
    }

    #[test]
    fn adopt_drop_attach_detach() {
        let mut app = App::new();
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
        let mut app = App::new();
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
        let mut app = App::new();
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
        parent.mark_needs_layout(&mut app);
        child.mark_needs_layout(&mut app);
        order.borrow_mut().clear();
        owner.flush_layout(&mut app);
        assert_eq!(*order.borrow(), ["parent", "child"]);
    }

    #[test]
    fn mark_needs_layout_requests_visual_update() {
        let mut app = App::new();
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
        node.mark_needs_layout(&mut app);
        assert_eq!(updates.get(), 1);
    }
}

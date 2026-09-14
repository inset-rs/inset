//! Port of Flutter's `rendering/layer.dart`.
//!
//! The layer tree: what a repaint boundary's recording is between paint and compositing, and
//! what `RenderView.compositeFrame` uploads through a [`SceneBuilder`]. A layer is an arena
//! object; edges are [`AnyLayer`] / [`AnyContainerLayer`] and lifetime is Flutter's
//! [`LayerHandle`] count, which ends in `dispose` and `App::destroy`.
//!
//! No `EngineLayer`: valo composites a display list, so `addToScene` runs for every layer every
//! frame and the retained-rendering bookkeeping (`_needsAddToScene`, `markNeedsAddToScene`,
//! `alwaysNeedsAddToScene`, `addRetained`, `engineLayer`) has nothing to decide and is not
//! ported. See `PORTING.md`.

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use inset_embedder::{
    BlendMode, Clip, EngineLayer, FillRule, ImageFilter, Matrix4, Offset, Picture, RRect, Rect,
    Scene, SceneBuilder, Size,
};
use inset_foundation::{App, Handle, HandleId, RetainedHandle};
use inset_scheduler::{FrameCallback, SchedulerBinding};

/// Information collected for an annotation that is found in the layer tree.
///
/// See also:
///
///  * [`AnyLayer::find_all_annotations`], which creates and uses objects of this type.
#[derive(Debug)]
pub struct AnnotationEntry<T> {
    /// The annotation object that is found.
    pub annotation: Rc<T>,
    /// The target location described by the local coordinate space of the annotation object.
    pub local_position: Offset,
}

impl<T> AnnotationEntry<T> {
    /// Create an entry of found annotation by providing the object and related information.
    pub fn new(annotation: Rc<T>, local_position: Offset) -> AnnotationEntry<T> {
        AnnotationEntry {
            annotation,
            local_position,
        }
    }
}

/// Information collected about a list of annotations that are found in the layer tree.
///
/// See also:
///
///  * [`AnnotationEntry`], which are members of this type.
///  * [`AnyLayer::find_all_annotations`], which creates and uses an object of this type.
#[derive(Debug)]
pub struct AnnotationResult<T> {
    entries: Vec<AnnotationEntry<T>>,
}

impl<T> AnnotationResult<T> {
    /// An empty result.
    pub fn new() -> AnnotationResult<T> {
        AnnotationResult {
            entries: Vec::new(),
        }
    }

    /// Add a new entry to the end of the result.
    ///
    /// Usually, entries should be added in order from most specific to least specific,
    /// typically during an upward walk of the tree.
    pub fn add(&mut self, entry: AnnotationEntry<T>) {
        self.entries.push(entry);
    }

    /// The [`AnnotationEntry`] objects recorded.
    ///
    /// The first entry is the most specific, typically the one at the leaf of the tree.
    pub fn entries(&self) -> &[AnnotationEntry<T>] {
        &self.entries
    }

    /// The annotations recorded.
    ///
    /// The first entry is the most specific, typically the one at the leaf of the tree. It is
    /// similar to [`entries`](Self::entries) but does not contain other information.
    pub fn annotations(&self) -> impl Iterator<Item = &Rc<T>> {
        self.entries.iter().map(|entry| &entry.annotation)
    }
}

impl<T> Default for AnnotationResult<T> {
    fn default() -> AnnotationResult<T> {
        AnnotationResult::new()
    }
}

/// Dart's `AnnotationResult<S>` while a search walks the tree: the type it looks for and what
/// it has found, erased so [`Layer::find_annotations`] can sit in a vtable. [`AnyLayer::find`]
/// and [`AnyLayer::find_all_annotations`] turn it back into an [`AnnotationResult<S>`]. See
/// `PORTING.md`.
#[derive(Debug)]
pub struct AnnotationSearch {
    searched: TypeId,
    entries: Vec<(Rc<dyn Any>, Offset)>,
}

impl AnnotationSearch {
    fn new<S: 'static>() -> AnnotationSearch {
        AnnotationSearch {
            searched: TypeId::of::<S>(),
            entries: Vec::new(),
        }
    }

    /// Dart's `T == S`: whether `annotation` is exactly the type this search is for.
    pub fn accepts(&self, annotation: &dyn Any) -> bool {
        annotation.type_id() == self.searched
    }

    /// Dart's `result.add(AnnotationEntry(annotation:, localPosition:))`. Only for an
    /// annotation [`accepts`](Self::accepts) answered true for.
    pub fn add(&mut self, annotation: Rc<dyn Any>, local_position: Offset) {
        debug_assert!(self.accepts(&*annotation));
        self.entries.push((annotation, local_position));
    }

    /// Dart's `result.entries.isNotEmpty`, negated.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn into_result<S: 'static>(self) -> AnnotationResult<S> {
        let mut result = AnnotationResult::new();
        for (annotation, local_position) in self.entries {
            let annotation = annotation
                .downcast::<S>()
                .expect("an accepted annotation is the searched type");
            result.add(AnnotationEntry::new(annotation, local_position));
        }
        result
    }
}

/// The callback of [`AnyLayer::add_composition_callback`].
pub type CompositionCallback = Rc<dyn Fn(&mut App, AnyLayer)>;

static NEXT_CALLBACK_ID: AtomicU64 = AtomicU64::new(0);

/// Flutter's `Layer` fields, held by every layer under the field `layer`
/// ([`layer_accessors!`]).
pub struct LayerData {
    callbacks: Vec<(u64, CompositionCallback)>,
    composition_callback_count: i32,
    debug_mutations_locked: bool,
    /// Set when this layer is appended to a [`ContainerLayer`], and unset when it is removed.
    ///
    /// This cannot be set from `attach` or `detach` which is called when an entire subtree is
    /// attached to or detached from an owner. Layers may be appended to or removed from a
    /// [`ContainerLayer`] regardless of whether they are attached or detached, and detaching a
    /// layer from an owner does not imply that it has been removed from its parent.
    parent_handle: LayerHandle<AnyLayer>,
    /// Incremented by [`LayerHandle`].
    ref_count: u32,
    debug_disposed: bool,
    parent: Option<AnyContainerLayer>,
    owner: Option<HandleId>,
    depth: i32,
    next_sibling: Option<AnyLayer>,
    previous_sibling: Option<AnyLayer>,
    /// Whether `add_to_scene` must run for this layer: it changed since it was last added, or
    /// never was. `update_subtree_needs_add_to_scene` folds the descendants' in before a scene
    /// is built.
    needs_add_to_scene: bool,
    /// The list this layer's subtree recorded when it was last added, for
    /// [`SceneBuilder::add_retained`] while nothing in the subtree changed.
    engine_layer: Option<EngineLayer>,
}

impl LayerData {
    /// Flutter's `Layer()` constructor.
    pub const fn new() -> LayerData {
        LayerData {
            callbacks: Vec::new(),
            composition_callback_count: 0,
            debug_mutations_locked: false,
            parent_handle: LayerHandle::new(),
            ref_count: 0,
            debug_disposed: false,
            parent: None,
            owner: None,
            depth: 0,
            next_sibling: None,
            previous_sibling: None,
            needs_add_to_scene: true,
            engine_layer: None,
        }
    }
}

impl Default for LayerData {
    fn default() -> LayerData {
        LayerData::new()
    }
}

impl Debug for LayerData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layer")
            .field("owner", &self.owner)
            .field("handles", &self.ref_count)
            .finish_non_exhaustive()
    }
}

/// Implements the [`Layer`] field accessors for a `layer` field.
///
/// `layer_accessors!(container)` also records that the type `extends ContainerLayer`, so its
/// [`AnyLayer`] can be narrowed with [`AnyLayer::as_container_layer`].
#[macro_export]
macro_rules! layer_accessors {
    () => {
        fn layer_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::LayerData {
            &app.get(self).layer
        }
        fn layer_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::LayerData {
            &mut app.get_mut(self).layer
        }
    };
    (container) => {
        $crate::layer_accessors!();
        const AS_CONTAINER: Option<fn() -> &'static $crate::ContainerLayerVTable> =
            Some(|| const { &$crate::ContainerLayerVTable::of::<Self>() });
    };
}

/// A composited layer.
///
/// During painting, the render tree generates a tree of composited layers that are uploaded
/// into the engine and displayed by the compositor. This class is the base class for all
/// composited layers.
///
/// Most layers can have their properties mutated, and layers can be moved to different parents.
/// The scene must be explicitly recomposited after such changes are made; the layer tree does
/// not maintain its own dirty state.
///
/// To composite the tree, create a [`SceneBuilder`] object, pass it to the root layer's
/// [`AnyContainerLayer::build_scene`] and give the [`Scene`] to the view.
///
/// ## Memory
///
/// Layers retain resources between frames to speed up rendering. A layer will retain these
/// resources until all [`LayerHandle`]s referring to the layer have nulled out their references.
///
/// Layers must not be used after disposal. If a `RenderObject` needs to maintain a layer for
/// later usage, it must create a handle to that layer. This is handled automatically for the
/// `RenderObject.layer` property, but additional layers must use their own [`LayerHandle`].
///
/// Flutter's abstract `Layer`: the shared bodies are on [`LayerBase`]. A leaf implements this
/// trait; where Dart runs the inherited body it calls it by name, `LayerBase::attach(self, app,
/// owner)`.
pub trait Layer: Sized + 'static {
    /// Dart's base-class fields, held under the field `layer` ([`layer_accessors!`]).
    fn layer_data(self: Handle<Self>, app: &App) -> &LayerData;

    /// See [`layer_data`](Self::layer_data).
    fn layer_data_mut(self: Handle<Self>, app: &mut App) -> &mut LayerData;

    /// The container table when this type `extends ContainerLayer`; `layer_accessors!(container)`
    /// sets it. Dart's `layer is ContainerLayer`.
    const AS_CONTAINER: Option<fn() -> &'static ContainerLayerVTable> = None;

    /// Whether or not this layer, or any child layers, can be rasterized with `Scene.toImage`.
    ///
    /// If `false`, calling the above methods may yield an image which is incomplete.
    ///
    /// This value may change throughout the lifetime of the object, as the child layers
    /// themselves are added or removed.
    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        let _ = (self, app);
        true
    }

    /// Describes the clip that would be applied to contents of this layer, if any.
    fn describe_clip_bounds(self: Handle<Self>, app: &App) -> Option<Rect> {
        let _ = (self, app);
        None
    }

    /// Subclasses may override this to true to disable retained rendering.
    fn always_needs_add_to_scene(self: Handle<Self>, app: &App) -> bool {
        let _ = (self, app);
        false
    }

    /// Clears any retained resources that this layer holds.
    ///
    /// This method must dispose resources such as [`Picture`] objects. The layer is still usable
    /// after this call, but any graphics related resources it holds will need to be recreated.
    ///
    /// This method _only_ disposes resources for this layer. For example, if it is a
    /// [`ContainerLayer`], it does not dispose resources of any children. However,
    /// [`ContainerLayer`]s do remove any children they have when this method is called, and if
    /// this layer was the last holder of a removed child handle, the child may recursively clean
    /// up its resources.
    ///
    /// This method automatically gets called when all outstanding [`LayerHandle`]s are disposed.
    /// [`LayerHandle`] objects are typically held by the parent layer of this layer and any
    /// `RenderObject`s that participated in creating it.
    ///
    /// After calling this method, the object is unusable: the base body destroys the arena entry.
    /// An override ends with `LayerBase::dispose(self, app)`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        LayerBase::dispose(self, app);
    }

    /// Mark this layer as attached to the given owner.
    ///
    /// Typically called only from the parent's `attach` method, and by the owner to mark the
    /// root of a tree as attached.
    ///
    /// Subclasses with children should override this method to `attach` all their children to
    /// the same owner after calling the inherited method, `LayerBase::attach(self, app, owner)`.
    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        LayerBase::attach(self, app, owner);
    }

    /// Mark this layer as detached from its owner.
    ///
    /// Typically called only from the parent's `detach`, and by the owner to mark the root of a
    /// tree as detached.
    ///
    /// Subclasses with children should override this method to `detach` all their children
    /// after calling the inherited method, `LayerBase::detach(self, app)`.
    fn detach(self: Handle<Self>, app: &mut App) {
        LayerBase::detach(self, app);
    }

    /// Adjust the depth of this node's children, if any.
    ///
    /// Override this method in subclasses with child nodes to call
    /// [`AnyContainerLayer::redepth_child`] for each child. Do not call this method directly.
    fn redepth_children(self: Handle<Self>, app: &mut App) {
        // ContainerLayer provides an implementation since its the only one that
        // can actually have children.
        let _ = (self, app);
    }

    /// Search this layer and its subtree for annotations of the searched type at the location
    /// described by `local_position`.
    ///
    /// This method is called by the default implementation of [`AnyLayer::find`] and
    /// [`AnyLayer::find_all_annotations`]. Override this method to customize how the layer
    /// should search for annotations, or if the layer has its own annotations to add.
    ///
    /// The default implementation always returns `false`, which means neither the layer nor its
    /// children has annotations, and the annotation search is not absorbed either.
    ///
    /// The annotations are searched by first visiting each child recursively, then this layer,
    /// resulting in an order from visually front to back. New annotations found during the walk
    /// are added to the tail of `result`.
    ///
    /// The `only_first` parameter indicates that, if true, the search will stop when it finds
    /// the first qualified annotation; otherwise, it will walk the entire subtree.
    ///
    /// The return value indicates the opacity of this layer and its subtree at this position. If
    /// it returns true, then this layer's parent should skip the children behind this layer. The
    /// return value does not affect whether the parent adds its own annotations.
    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let _ = (self, app, result, local_position, only_first);
        false
    }

    /// Override this method to upload this layer to the engine.
    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder);

    /// Dart's `_fireCompositionCallbacks`; [`ContainerLayer`] recurses into its children.
    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        LayerBase::fire_composition_callbacks(self, app, include_children);
    }
}

/// The bodies of Flutter's `Layer` that a subclass calls as `super`: `LayerBase::dispose(self,
/// app)` where Dart writes `super.dispose()`. The virtual call is on [`AnyLayer`].
pub trait LayerBase: Layer {
    /// `Layer.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        let data = self.layer_data_mut(app);
        debug_assert!(!data.debug_mutations_locked);
        debug_assert!(
            !data.debug_disposed,
            "Layers must only be disposed once. This is typically handled by LayerHandle. \
             Subclasses should not directly call dispose, except to call LayerBase::dispose in \
             an overridden dispose method. Tests must only call dispose once."
        );
        debug_assert!(
            data.ref_count == 0,
            "Do not directly call dispose on a layer. Instead, use a LayerHandle."
        );
        data.debug_disposed = true;
        // Dart's engine layer is dropped here; the arena entry goes with it.
        app.destroy(self);
    }

    /// `Layer.attach`.
    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        let data = self.layer_data_mut(app);
        debug_assert!(data.owner.is_none());
        data.owner = Some(owner);
    }

    /// `Layer.detach`.
    fn detach(self: Handle<Self>, app: &mut App) {
        let data = self.layer_data_mut(app);
        debug_assert!(data.owner.is_some());
        data.owner = None;
        let this = self.as_layer();
        debug_assert!(
            this.parent(app)
                .is_none_or(|parent| this.attached(app) == parent.as_layer().attached(app))
        );
    }

    /// `Layer._fireCompositionCallbacks`.
    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        let _ = include_children;
        let this = self.as_layer();
        let callbacks: Vec<CompositionCallback> = self
            .layer_data(app)
            .callbacks
            .iter()
            .map(|(_, callback)| Rc::clone(callback))
            .collect();
        for callback in callbacks {
            this.run_composition_callback(app, &callback);
        }
    }
}

impl<T: Layer> LayerBase for T {}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

/// The vtable of an erased [`AnyLayer`]: one `&'static` table per type, built from the trait
/// impl by [`Layer::as_layer`]. Copied out of the handle before a virtual call, so the slot is
/// not borrowed across it.
///
/// Public only so [`layer_accessors!`] can name it from another crate.
#[doc(hidden)]
pub struct LayerVTable {
    layer_data: fn(&App, HandleId) -> &LayerData,
    layer_data_mut: fn(&mut App, HandleId) -> &mut LayerData,
    supports_rasterization: fn(&App, HandleId) -> bool,
    describe_clip_bounds: fn(&App, HandleId) -> Option<Rect>,
    always_needs_add_to_scene: fn(&App, HandleId) -> bool,
    dispose: fn(&mut App, HandleId),
    attach: fn(&mut App, HandleId, HandleId),
    detach: fn(&mut App, HandleId),
    redepth_children: fn(&mut App, HandleId),
    find_annotations: fn(&App, HandleId, &mut AnnotationSearch, Offset, bool) -> bool,
    add_to_scene: fn(&mut App, HandleId, &mut SceneBuilder),
    fire_composition_callbacks: fn(&mut App, HandleId, bool),
    as_container: Option<fn() -> &'static ContainerLayerVTable>,
}

impl LayerVTable {
    pub const fn of<T: Layer>() -> LayerVTable {
        LayerVTable {
            layer_data: |app, id| T::layer_data(resolve(id), app),
            layer_data_mut: |app, id| T::layer_data_mut(resolve(id), app),
            supports_rasterization: |app, id| T::supports_rasterization(resolve(id), app),
            describe_clip_bounds: |app, id| T::describe_clip_bounds(resolve(id), app),
            always_needs_add_to_scene: |app, id| T::always_needs_add_to_scene(resolve(id), app),
            dispose: |app, id| T::dispose(resolve(id), app),
            attach: |app, id, owner| T::attach(resolve(id), app, owner),
            detach: |app, id| T::detach(resolve(id), app),
            redepth_children: |app, id| T::redepth_children(resolve(id), app),
            find_annotations: |app, id, result, local_position, only_first| {
                T::find_annotations(resolve(id), app, result, local_position, only_first)
            },
            add_to_scene: |app, id, builder| T::add_to_scene(resolve(id), app, builder),
            fire_composition_callbacks: |app, id, include_children| {
                T::fire_composition_callbacks(resolve(id), app, include_children)
            },
            as_container: T::AS_CONTAINER,
        }
    }
}

/// Erased `Layer`: one identity and a static vtable, the fat pointer rustc cannot build for an
/// arena id. No lease.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyLayer {
    id: HandleId,
    vtable: &'static LayerVTable,
}

impl PartialEq for AnyLayer {
    fn eq(&self, other: &AnyLayer) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyLayer {}

impl Hash for AnyLayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<AnyLayer> for HandleId {
    fn from(layer: AnyLayer) -> HandleId {
        layer.id
    }
}

impl Debug for AnyLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyLayer({:?})", self.id)
    }
}

impl AnyLayer {
    pub fn id(self) -> HandleId {
        self.id
    }

    fn data(self, app: &App) -> &LayerData {
        (self.vtable.layer_data)(app, self.id)
    }

    fn data_mut(self, app: &mut App) -> &mut LayerData {
        (self.vtable.layer_data_mut)(app, self.id)
    }

    /// Dart's `layer as ContainerLayer`, as an `Option`.
    pub fn as_container_layer(self) -> Option<AnyContainerLayer> {
        self.vtable
            .as_container
            .map(|vtable| AnyContainerLayer::from_vtable(self.id, vtable()))
    }

    /// Whether the subtree rooted at this layer has any composition callback observers.
    ///
    /// This only evaluates to true if the subtree rooted at this node has observers. For
    /// example, it may evaluate to true on a parent node but false on a child if the parent has
    /// observers but the child does not.
    pub fn subtree_has_composition_callbacks(self, app: &App) -> bool {
        self.data(app).composition_callback_count > 0
    }

    fn update_subtree_composition_observer_count(self, app: &mut App, delta: i32) {
        debug_assert!(delta != 0);
        let data = self.data_mut(app);
        data.composition_callback_count += delta;
        debug_assert!(data.composition_callback_count >= 0);
        if let Some(parent) = data.parent {
            parent
                .as_layer()
                .update_subtree_composition_observer_count(app, delta);
        }
    }

    /// See [`Layer::fire_composition_callbacks`].
    pub(crate) fn fire_composition_callbacks(self, app: &mut App, include_children: bool) {
        (self.vtable.fire_composition_callbacks)(app, self.id, include_children);
    }

    /// The closure Dart stores in `_callbacks`: the callback between the mutation-lock asserts.
    fn run_composition_callback(self, app: &mut App, callback: &CompositionCallback) {
        if cfg!(debug_assertions) {
            self.data_mut(app).debug_mutations_locked = true;
        }
        callback(app, self);
        if cfg!(debug_assertions) {
            self.data_mut(app).debug_mutations_locked = false;
        }
    }

    /// See [`Layer::supports_rasterization`].
    pub fn supports_rasterization(self, app: &App) -> bool {
        (self.vtable.supports_rasterization)(app, self.id)
    }

    /// See [`Layer::describe_clip_bounds`].
    pub fn describe_clip_bounds(self, app: &App) -> Option<Rect> {
        (self.vtable.describe_clip_bounds)(app, self.id)
    }

    /// Adds a callback for when the layer tree that this layer is part of gets composited, or
    /// when it is detached and will not be rendered again.
    ///
    /// The callback receives a reference to this layer. The recipient must not mutate the layer
    /// during the scope of the callback, but may traverse the tree to find information about the
    /// current transform or clip. The layer may not be attached anymore in this state, but even
    /// if it is detached it may still have an also detached parent it can visit.
    ///
    /// If new callbacks are added or removed within the callback, the new callbacks will fire
    /// (or stop firing) on the _next_ compositing event.
    ///
    /// Composition callbacks are useful in place of pushing a layer that would otherwise try to
    /// observe the layer tree without actually affecting compositing. For example, a composition
    /// callback may be used to observe the total transform and clip of the current container
    /// layer to determine whether a render object drawn into it is visible or not.
    ///
    /// Calling the returned callback will remove `callback` from the composition callbacks.
    pub fn add_composition_callback(
        self,
        app: &mut App,
        callback: CompositionCallback,
    ) -> Box<dyn FnOnce(&mut App)> {
        self.update_subtree_composition_observer_count(app, 1);
        let callback_id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::Relaxed) + 1;
        self.data_mut(app).callbacks.push((callback_id, callback));
        Box::new(move |app: &mut App| {
            // Dart's closure still finds a disposed layer; here dispose freed the entry.
            if !app.contains(self.id) {
                return;
            }
            let index = self
                .data(app)
                .callbacks
                .iter()
                .position(|(id, _)| *id == callback_id);
            debug_assert!(self.debug_disposed(app) || index.is_some());
            if let Some(index) = index {
                self.data_mut(app).callbacks.remove(index);
            }
            self.update_subtree_composition_observer_count(app, -1);
        })
    }

    /// If asserts are enabled, returns whether `dispose` has been called since the last time any
    /// retained resources were created.
    ///
    /// A disposed layer's arena entry is gone, so a stale handle answers true as well.
    pub fn debug_disposed(self, app: &App) -> bool {
        !app.contains(self.id) || self.data(app).debug_disposed
    }

    /// Called by [`LayerHandle`].
    fn unref(self, app: &mut App) {
        let data = self.data_mut(app);
        debug_assert!(!data.debug_mutations_locked);
        debug_assert!(data.ref_count > 0);
        data.ref_count -= 1;
        if data.ref_count == 0 {
            self.dispose(app);
        }
    }

    /// Returns the number of objects holding a [`LayerHandle`] to this layer.
    pub fn debug_handle_count(self, app: &App) -> u32 {
        self.data(app).ref_count
    }

    /// See [`Layer::dispose`].
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }

    /// This layer's parent in the layer tree.
    ///
    /// The parent of the root node in the layer tree is `None`.
    ///
    /// Only subclasses of [`ContainerLayer`] can have children in the layer tree. All other
    /// layer classes are used for leaves in the layer tree.
    pub fn parent(self, app: &App) -> Option<AnyContainerLayer> {
        self.data(app).parent
    }

    /// The owner for this layer (`None` if unattached).
    ///
    /// The entire layer tree that this layer belongs to will have the same owner.
    ///
    /// Typically the owner is a `RenderView`.
    pub fn owner(self, app: &App) -> Option<HandleId> {
        self.data(app).owner
    }

    /// Whether the layer tree containing this layer is attached to an owner.
    ///
    /// This becomes true during the call to `attach`.
    ///
    /// This becomes false during the call to `detach`.
    pub fn attached(self, app: &App) -> bool {
        self.data(app).owner.is_some()
    }

    /// See [`Layer::attach`].
    pub fn attach(self, app: &mut App, owner: HandleId) {
        (self.vtable.attach)(app, self.id, owner);
    }

    /// See [`Layer::detach`].
    pub fn detach(self, app: &mut App) {
        (self.vtable.detach)(app, self.id);
    }

    /// The depth of this layer in the layer tree.
    ///
    /// The depth of nodes in a tree monotonically increases as you traverse down the tree.
    /// There's no guarantee regarding depth between siblings.
    ///
    /// The depth is used to ensure that nodes are processed in depth order.
    pub fn depth(self, app: &App) -> i32 {
        self.data(app).depth
    }

    /// See [`Layer::redepth_children`].
    pub fn redepth_children(self, app: &mut App) {
        (self.vtable.redepth_children)(app, self.id);
    }

    /// This layer's next sibling in the parent layer's child list.
    pub fn next_sibling(self, app: &App) -> Option<AnyLayer> {
        self.data(app).next_sibling
    }

    /// This layer's previous sibling in the parent layer's child list.
    pub fn previous_sibling(self, app: &App) -> Option<AnyLayer> {
        self.data(app).previous_sibling
    }

    /// Removes this layer from its parent layer's child list.
    ///
    /// This has no effect if the layer's parent is already `None`.
    pub fn remove(self, app: &mut App) {
        debug_assert!(!self.data(app).debug_mutations_locked);
        if let Some(parent) = self.parent(app) {
            parent.remove_child(app, self);
        }
    }

    /// See [`Layer::find_annotations`].
    pub fn find_annotations(
        self,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        (self.vtable.find_annotations)(app, self.id, result, local_position, only_first)
    }

    /// Search this layer and its subtree for the first annotation of type `S` under the point
    /// described by `local_position`.
    ///
    /// Returns `None` if no matching annotations are found.
    ///
    /// By default this method calls [`find_annotations`](Self::find_annotations) with
    /// `only_first: true` and returns the annotation of the first result. Prefer overriding
    /// [`Layer::find_annotations`] instead of this method, because during an annotation search,
    /// only `find_annotations` is recursively called, while custom behavior in this method is
    /// ignored.
    ///
    /// See also:
    ///
    ///  * [`find_all_annotations`](Self::find_all_annotations), which is similar but returns all
    ///    annotations found at the given position.
    ///  * `AnnotatedRegionLayer`, for placing values in the layer tree.
    pub fn find<S: 'static>(self, app: &App, local_position: Offset) -> Option<Rc<S>> {
        let mut result = AnnotationSearch::new::<S>();
        self.find_annotations(app, &mut result, local_position, true);
        result
            .into_result::<S>()
            .entries
            .into_iter()
            .next()
            .map(|entry| entry.annotation)
    }

    /// Search this layer and its subtree for all annotations of type `S` under the point
    /// described by `local_position`.
    ///
    /// Returns a result with empty entries if no matching annotations are found.
    ///
    /// By default this method calls [`find_annotations`](Self::find_annotations) with
    /// `only_first: false` and returns the annotations of its result. Prefer overriding
    /// [`Layer::find_annotations`] instead of this method, because during an annotation search,
    /// only `find_annotations` is recursively called, while custom behavior in this method is
    /// ignored.
    ///
    /// See also:
    ///
    ///  * [`find`](Self::find), which is similar but returns the first annotation found at the
    ///    given position.
    ///  * `AnnotatedRegionLayer`, for placing values in the layer tree.
    pub fn find_all_annotations<S: 'static>(
        self,
        app: &App,
        local_position: Offset,
    ) -> AnnotationResult<S> {
        let mut result = AnnotationSearch::new::<S>();
        self.find_annotations(app, &mut result, local_position, false);
        result.into_result::<S>()
    }

    /// See [`Layer::add_to_scene`].
    pub fn add_to_scene(self, app: &mut App, builder: &mut SceneBuilder) {
        (self.vtable.add_to_scene)(app, self.id, builder);
    }

    /// Whether `add_to_scene` must run for this layer when a scene is next built: it changed
    /// since it was last added, or never was, or a descendant did as far as
    /// [`update_subtree_needs_add_to_scene`](Self::update_subtree_needs_add_to_scene) has
    /// found out.
    pub fn needs_add_to_scene(self, app: &App) -> bool {
        self.data(app).needs_add_to_scene
    }

    /// Mark that this layer has changed and `add_to_scene` needs to be called.
    pub fn mark_needs_add_to_scene(self, app: &mut App) {
        debug_assert!(!self.data(app).debug_mutations_locked);
        debug_assert!(
            !self.always_needs_add_to_scene(app),
            "a layer with always_needs_add_to_scene set called mark_needs_add_to_scene"
        );
        debug_assert!(!self.data(app).debug_disposed);
        // Already marked. Short-circuit.
        if self.data(app).needs_add_to_scene {
            return;
        }
        self.data_mut(app).needs_add_to_scene = true;
    }

    /// See [`Layer::always_needs_add_to_scene`].
    pub fn always_needs_add_to_scene(self, app: &App) -> bool {
        (self.vtable.always_needs_add_to_scene)(app, self.id)
    }

    /// The engine layer used to render this layer: the list its subtree recorded when it was
    /// last added to a scene, which [`SceneBuilder::add_retained`] embeds again while nothing
    /// in the subtree changed.
    pub fn engine_layer(self, app: &App) -> Option<EngineLayer> {
        self.data(app).engine_layer.clone()
    }

    /// Sets the engine layer used to render this layer.
    ///
    /// Typically this is set to the value [`SceneBuilder::pop`] returned in `add_to_scene`.
    pub fn set_engine_layer(self, app: &mut App, value: Option<EngineLayer>) {
        debug_assert!(!self.data(app).debug_mutations_locked);
        debug_assert!(!self.data(app).debug_disposed);
        self.data_mut(app).engine_layer = value;
        if !self.always_needs_add_to_scene(app) {
            // The parent must record a new list to embed this one in, and so is marked as
            // needing `add_to_scene`. When the whole tree is rendered the parent is adding
            // itself already and clears the flag when it is done; when an interior layer is
            // rendered on its own (an `OffsetLayer::to_image`) the mark waits for the frame
            // that next renders the parent.
            if let Some(parent) = self.parent(app)
                && !parent.as_layer().always_needs_add_to_scene(app)
            {
                parent.as_layer().mark_needs_add_to_scene(app);
            }
        }
    }

    /// Traverses the layer subtree starting from this layer and determines whether it needs
    /// `add_to_scene`.
    ///
    /// A layer needs `add_to_scene` if any of the following is true:
    ///
    /// - [`always_needs_add_to_scene`](Self::always_needs_add_to_scene) is true.
    /// - [`mark_needs_add_to_scene`](Self::mark_needs_add_to_scene) has been called.
    /// - Any of its descendants need `add_to_scene`.
    ///
    /// Dart's `ContainerLayer` override recurses into the children; here the erased handle
    /// knows whether it is a container.
    pub fn update_subtree_needs_add_to_scene(self, app: &mut App) {
        debug_assert!(!self.data(app).debug_mutations_locked);
        let always = self.always_needs_add_to_scene(app);
        let data = self.data_mut(app);
        data.needs_add_to_scene = data.needs_add_to_scene || always;
        let Some(container) = self.as_container_layer() else {
            return;
        };
        let mut child = container.first_child(app);
        while let Some(layer) = child {
            layer.update_subtree_needs_add_to_scene(app);
            if layer.data(app).needs_add_to_scene {
                self.data_mut(app).needs_add_to_scene = true;
            }
            child = layer.next_sibling(app);
        }
    }

    /// Dart's `_addToSceneWithRetainedRendering`: the retained engine layer where the subtree
    /// has not changed, `add_to_scene` otherwise.
    pub fn add_to_scene_with_retained_rendering(self, app: &mut App, builder: &mut SceneBuilder) {
        debug_assert!(!self.data(app).debug_mutations_locked);
        // There can't be a loop by adding a retained layer subtree whose needs_add_to_scene is
        // false: a retained layer appended to one of its own descendants changes that
        // descendant's children, which sets the flag.
        if !self.data(app).needs_add_to_scene
            && let Some(engine_layer) = self.engine_layer(app)
        {
            builder.add_retained(&engine_layer);
            return;
        }
        self.add_to_scene(app, builder);
        // Clearing the flag after `add_to_scene`, not before: it calls the children's, which
        // may mark this layer.
        self.data_mut(app).needs_add_to_scene = false;
    }
}

/// A layer handle of any concreteness — [`AnyLayer`], [`AnyContainerLayer`], [`AnyOffsetLayer`]
/// or a typed `Handle<L>` — for what a [`LayerHandle`] can hold. Dart's `T extends Layer`.
pub trait ErasedLayer: Copy + PartialEq + Debug + 'static {
    /// Dart's implicit upcast to `Layer`.
    fn as_layer(self) -> AnyLayer;
}

impl ErasedLayer for AnyLayer {
    fn as_layer(self) -> AnyLayer {
        self
    }
}

/// Dart's implicit upcast of a concrete layer to `Layer`: the type-erased handle, through the
/// table built from the type's `impl Layer`.
impl<T: Layer> ErasedLayer for Handle<T> {
    fn as_layer(self) -> AnyLayer {
        AnyLayer {
            id: self.id(),
            vtable: const { &LayerVTable::of::<T>() },
        }
    }
}

/// A handle to prevent a [`Layer`]'s platform graphics resources from being disposed.
///
/// [`Layer`] objects retain native resources such as [`Picture`] objects. These objects may in
/// turn retain large chunks of texture memory, either directly or indirectly.
///
/// The layer's native resources must be retained as long as there is some object that can add
/// it to a scene. Typically, this is either its parent or an undisposed `RenderObject` that will
/// append it to a [`ContainerLayer`]. Layers automatically hold a handle to their children, and
/// `RenderObject`s automatically hold a handle to their `RenderObject.layer` as well as any
/// [`PictureLayer`]s that they paint into using the `PaintingContext.canvas`. A layer
/// automatically releases its resources once at least one handle has been acquired and all
/// handles have been disposed. `RenderObject`s that create additional layer objects must
/// manually manage the handles for that layer similarly to the implementation of
/// `RenderObject.layer`.
///
/// If a `RenderObject` creates layers in addition to its `RenderObject.layer` and it intends to
/// reuse those layers separately from `RenderObject.layer`, it must create a handle to that
/// layer and dispose of it when the layer is no longer needed. For example, if it re-creates or
/// nulls out an existing layer in `RenderObject.paint`, it should dispose of the handle to the
/// old layer. It should also dispose of any layer handles it holds in `RenderObject.dispose`.
///
/// To dispose of a layer handle, set its layer to `None` with [`set_layer`](Self::set_layer).
/// The handle lives inside an arena object, so the setter reaches it through the `App` rather
/// than through `&mut self`; a handle dropped without clearing keeps its layer until the `App`
/// goes, as a Dart handle that is never nulled would. See `PORTING.md`.
pub struct LayerHandle<T> {
    layer: Option<T>,
}

impl<T> LayerHandle<T> {
    /// Create a new layer handle referencing no [`Layer`]. Dart's `LayerHandle([layer])` with
    /// a layer is `new` then [`set_layer`](Self::set_layer).
    pub const fn new() -> LayerHandle<T> {
        LayerHandle { layer: None }
    }
}

impl<T: ErasedLayer> LayerHandle<T> {
    /// The [`Layer`] whose resources this object keeps alive.
    pub fn layer(&self) -> Option<T> {
        self.layer
    }

    /// Dart's `handle.layer = layer` for the handle `at` reaches inside `app`.
    ///
    /// Setting a new value or `None` will dispose the previously held layer if there are no
    /// other open handles to that layer.
    ///
    /// # Panics
    ///
    /// In debug builds, if `layer` is already disposed.
    pub fn set_layer(
        app: &mut App,
        at: impl Fn(&mut App) -> &mut LayerHandle<T>,
        layer: Option<T>,
    ) {
        debug_assert!(
            layer.is_none_or(|layer| !layer.as_layer().debug_disposed(app)),
            "Attempted to create a handle to an already disposed layer: {layer:?}."
        );
        let old = at(app).layer;
        if layer == old {
            return;
        }
        // The write comes before the unref, not after as in Dart: a layer's `parent_handle`
        // lives in the layer it holds, and the unref may destroy that entry.
        at(app).layer = layer;
        if let Some(layer) = layer {
            layer.as_layer().data_mut(app).ref_count += 1;
        }
        if let Some(old) = old {
            old.as_layer().unref(app);
        }
    }
}

impl<T> Default for LayerHandle<T> {
    fn default() -> LayerHandle<T> {
        LayerHandle::new()
    }
}

impl<T: Debug> Debug for LayerHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.layer {
            Some(layer) => write!(f, "LayerHandle({layer:?})"),
            None => write!(f, "LayerHandle(DISPOSED)"),
        }
    }
}

/// A composited layer containing a [`Picture`].
///
/// Picture layers are always leaves in the layer tree. They are also responsible for disposing
/// of the [`Picture`] object they hold. This is typically done when their parent and all
/// `RenderObject`s that participated in painting the picture have been disposed.
pub struct PictureLayer {
    layer: LayerData,
    /// The bounds that were used for the canvas that drew this layer's picture.
    ///
    /// This is purely advisory. It can help debug why certain drawing commands are being culled.
    pub canvas_bounds: Rect,
    picture: Option<Arc<Picture>>,
    is_complex_hint: bool,
    will_change_hint: bool,
}

impl PictureLayer {
    /// Creates a leaf layer for the layer tree.
    pub fn new(app: &mut App, canvas_bounds: Rect) -> Handle<PictureLayer> {
        app.create(PictureLayer {
            layer: LayerData::new(),
            canvas_bounds,
            picture: None,
            is_complex_hint: false,
            will_change_hint: false,
        })
    }

    /// The picture recorded for this layer.
    ///
    /// The picture's coordinate system matches this layer's coordinate system.
    pub fn picture(self: Handle<Self>, app: &App) -> Option<&Arc<Picture>> {
        app.get(self).picture.as_ref()
    }

    /// See [`picture`](Self::picture). Dart's `_picture?.dispose()` is the drop of the old one.
    pub fn set_picture(self: Handle<Self>, app: &mut App, picture: Option<Arc<Picture>>) {
        debug_assert!(!app.get(self).layer.debug_disposed);
        self.as_layer().mark_needs_add_to_scene(app);
        app.get_mut(self).picture = picture;
    }

    /// Hints that the painting in this layer is complex and would benefit from caching.
    ///
    /// If this hint is not set, the compositor will apply its own heuristics to decide whether
    /// the this layer is complex enough to benefit from caching.
    pub fn is_complex_hint(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_complex_hint
    }

    /// See [`is_complex_hint`](Self::is_complex_hint).
    pub fn set_is_complex_hint(self: Handle<Self>, app: &mut App, value: bool) {
        if value != app.get(self).is_complex_hint {
            app.get_mut(self).is_complex_hint = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }

    /// Hints that the painting in this layer is likely to change next frame.
    ///
    /// This hint tells the compositor not to cache this layer because the cache will not be used
    /// in the future. If this hint is not set, the compositor will apply its own heuristics to
    /// decide whether this layer is likely to be reused in the future.
    pub fn will_change_hint(self: Handle<Self>, app: &App) -> bool {
        app.get(self).will_change_hint
    }

    /// See [`will_change_hint`](Self::will_change_hint).
    pub fn set_will_change_hint(self: Handle<Self>, app: &mut App, value: bool) {
        if value != app.get(self).will_change_hint {
            app.get_mut(self).will_change_hint = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

impl Layer for PictureLayer {
    crate::layer_accessors!();

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.set_picture(app, None); // Will dispose _picture.
        LayerBase::dispose(self, app);
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        let this = app.get(self);
        let picture = this
            .picture
            .as_ref()
            .expect("a PictureLayer has a picture when it is added to the scene");
        builder.add_picture(
            Offset::ZERO,
            picture,
            this.is_complex_hint,
            this.will_change_hint,
        );
    }

    fn find_annotations(
        self: Handle<Self>,
        _app: &App,
        _result: &mut AnnotationSearch,
        _local_position: Offset,
        _only_first: bool,
    ) -> bool {
        false
    }
}

/// Flutter's `ContainerLayer` fields, held under the field `container_layer`
/// ([`container_layer_accessors!`]).
#[derive(Debug, Default)]
pub struct ContainerLayerData {
    first_child: Option<AnyLayer>,
    last_child: Option<AnyLayer>,
}

impl ContainerLayerData {
    /// No children.
    pub const fn new() -> ContainerLayerData {
        ContainerLayerData {
            first_child: None,
            last_child: None,
        }
    }
}

/// Implements the [`ContainerLayer`] field accessors for a `container_layer` field.
///
/// `container_layer_accessors!(offset)` also records that the type `extends OffsetLayer`, so
/// its [`AnyContainerLayer`] can be narrowed with [`AnyContainerLayer::as_offset_layer`].
#[macro_export]
macro_rules! container_layer_accessors {
    () => {
        fn container_layer_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::ContainerLayerData {
            &app.get(self).container_layer
        }
        fn container_layer_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::ContainerLayerData {
            &mut app.get_mut(self).container_layer
        }
    };
    (offset) => {
        $crate::container_layer_accessors!();
        const AS_OFFSET: Option<fn() -> &'static $crate::OffsetLayerVTable> =
            Some(|| const { &$crate::OffsetLayerVTable::of::<Self>() });
    };
}

/// A composited layer that has a list of children.
///
/// A [`ContainerLayer`] instance merely takes a list of children and inserts them into the
/// composited rendering in order. There are subclasses of [`ContainerLayer`] which apply more
/// elaborate effects in the process.
///
/// Flutter's `ContainerLayer`: the child list lives on [`AnyContainerLayer`]; this trait holds
/// the virtual `apply_transform` and the bodies of the `Layer` methods it overrides. A leaf's
/// `impl Layer` forwards to them: `fn attach(..) { ContainerLayer::attach(self, app, owner) }`.
pub trait ContainerLayer: Layer {
    /// Mixin field access.
    fn container_layer_data(self: Handle<Self>, app: &App) -> &ContainerLayerData;

    /// See [`container_layer_data`](Self::container_layer_data).
    fn container_layer_data_mut(self: Handle<Self>, app: &mut App) -> &mut ContainerLayerData;

    /// The offset table when this type `extends OffsetLayer`;
    /// `container_layer_accessors!(offset)` sets it. Dart's `layer is OffsetLayer`.
    const AS_OFFSET: Option<fn() -> &'static OffsetLayerVTable> = None;

    /// Dart's implicit upcast of a concrete layer to `ContainerLayer`.
    fn as_container_layer(self: Handle<Self>) -> AnyContainerLayer {
        AnyContainerLayer {
            id: self.id(),
            vtable: const { &ContainerLayerVTable::of::<Self>() },
        }
    }

    /// Applies the transform that would be applied when compositing the given child to the
    /// given matrix.
    ///
    /// Specifically, this should apply the transform that is applied to child's _origin_. When
    /// using `apply_transform` with a chain of layers, results will be unreliable unless the
    /// deepest layer in the chain collapses the `layerOffset` in `add_to_scene` to zero, meaning
    /// that it passes [`Offset::ZERO`] to its children, and bakes any incoming `layerOffset` into
    /// the [`SceneBuilder`] as (for instance) a transform (which is then also included in the
    /// transformation applied by `apply_transform`).
    ///
    /// This method is only valid immediately after `add_to_scene` has been called, before any of
    /// the properties have been changed.
    ///
    /// The default implementation does nothing, since [`ContainerLayer`], by default, composites
    /// its children at the origin of the [`ContainerLayer`] itself.
    ///
    /// The `child` argument should generally not be `None`, since in principle a layer could
    /// transform each child independently. However, certain layers may explicitly allow `None`
    /// as a value, for example if they know that they transform all their children identically.
    ///
    /// Used by `FollowerLayer` to transform its child to a `LeaderLayer`'s position.
    fn apply_transform(
        self: Handle<Self>,
        app: &App,
        child: Option<AnyLayer>,
        transform: &mut Matrix4,
    ) {
        let _ = (self, app, transform);
        debug_assert!(child.is_some());
    }

    /// `ContainerLayer._fireCompositionCallbacks`.
    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        LayerBase::fire_composition_callbacks(self, app, include_children);
        if !include_children {
            return;
        }
        let mut child = self.as_container_layer().first_child(app);
        while let Some(layer) = child {
            layer.fire_composition_callbacks(app, include_children);
            child = layer.next_sibling(app);
        }
    }

    /// `ContainerLayer.supportsRasterization`.
    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        let mut child = self.as_container_layer().last_child(app);
        while let Some(layer) = child {
            if !layer.supports_rasterization(app) {
                return false;
            }
            child = layer.previous_sibling(app);
        }
        true
    }

    /// `ContainerLayer.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.as_container_layer().remove_all_children(app);
        self.layer_data_mut(app).callbacks.clear();
        LayerBase::dispose(self, app);
    }

    /// `ContainerLayer.findAnnotations`.
    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let mut child = self.as_container_layer().last_child(app);
        while let Some(layer) = child {
            let is_absorbed = layer.find_annotations(app, result, local_position, only_first);
            if is_absorbed {
                return true;
            }
            if only_first && !result.is_empty() {
                return is_absorbed;
            }
            child = layer.previous_sibling(app);
        }
        false
    }

    /// `ContainerLayer.attach`.
    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        debug_assert!(!self.layer_data(app).debug_mutations_locked);
        LayerBase::attach(self, app, owner);
        let mut child = self.as_container_layer().first_child(app);
        while let Some(layer) = child {
            layer.attach(app, owner);
            child = layer.next_sibling(app);
        }
    }

    /// `ContainerLayer.detach`.
    fn detach(self: Handle<Self>, app: &mut App) {
        debug_assert!(!self.layer_data(app).debug_mutations_locked);
        LayerBase::detach(self, app);
        let mut child = self.as_container_layer().first_child(app);
        while let Some(layer) = child {
            layer.detach(app);
            child = layer.next_sibling(app);
        }
        // Detach indicates that we may never be composited again. Clients
        // interested in observing composition need to get an update here because
        // they might otherwise never get another one even though the layer is no
        // longer visible.
        //
        // Children fired them already in child.detach().
        self.as_layer().fire_composition_callbacks(app, false);
    }

    /// `ContainerLayer.redepthChildren`.
    fn redepth_children(self: Handle<Self>, app: &mut App) {
        let this = self.as_container_layer();
        let mut child = this.first_child(app);
        while let Some(layer) = child {
            this.redepth_child(app, layer);
            child = layer.next_sibling(app);
        }
    }

    /// `ContainerLayer.addToScene`.
    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        self.as_container_layer()
            .add_children_to_scene(app, builder);
    }
}

/// The vtable of an [`AnyContainerLayer`]: the layer vtable plus the container accessors.
///
/// Public only so [`layer_accessors!`] can name it from another crate.
#[doc(hidden)]
pub struct ContainerLayerVTable {
    layer: LayerVTable,
    container_data: fn(&App, HandleId) -> &ContainerLayerData,
    container_data_mut: fn(&mut App, HandleId) -> &mut ContainerLayerData,
    apply_transform: fn(&App, HandleId, Option<AnyLayer>, &mut Matrix4),
    as_offset: Option<fn() -> &'static OffsetLayerVTable>,
}

impl ContainerLayerVTable {
    pub const fn of<T: ContainerLayer>() -> ContainerLayerVTable {
        ContainerLayerVTable {
            layer: LayerVTable::of::<T>(),
            container_data: |app, id| T::container_layer_data(resolve(id), app),
            container_data_mut: |app, id| T::container_layer_data_mut(resolve(id), app),
            apply_transform: |app, id, child, transform| {
                T::apply_transform(resolve(id), app, child, transform)
            },
            as_offset: T::AS_OFFSET,
        }
    }
}

/// Erased `ContainerLayer`.
#[derive(Clone, Copy)]
pub struct AnyContainerLayer {
    id: HandleId,
    vtable: &'static ContainerLayerVTable,
}

impl PartialEq for AnyContainerLayer {
    fn eq(&self, other: &AnyContainerLayer) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyContainerLayer {}

impl Hash for AnyContainerLayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<AnyContainerLayer> for HandleId {
    fn from(layer: AnyContainerLayer) -> HandleId {
        layer.id
    }
}

impl Debug for AnyContainerLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyContainerLayer({:?})", self.id)
    }
}

impl ErasedLayer for AnyContainerLayer {
    fn as_layer(self) -> AnyLayer {
        AnyLayer {
            id: self.id,
            vtable: &self.vtable.layer,
        }
    }
}

impl AnyContainerLayer {
    fn from_vtable(id: HandleId, vtable: &'static ContainerLayerVTable) -> AnyContainerLayer {
        AnyContainerLayer { id, vtable }
    }

    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `layer as OffsetLayer`, as an `Option`.
    pub fn as_offset_layer(self) -> Option<AnyOffsetLayer> {
        self.vtable
            .as_offset
            .map(|vtable| AnyOffsetLayer::from_vtable(self.id, vtable()))
    }

    fn data(self, app: &App) -> &ContainerLayerData {
        (self.vtable.container_data)(app, self.id)
    }

    fn data_mut(self, app: &mut App) -> &mut ContainerLayerData {
        (self.vtable.container_data_mut)(app, self.id)
    }

    /// The first composited layer in this layer's child list.
    pub fn first_child(self, app: &App) -> Option<AnyLayer> {
        self.data(app).first_child
    }

    /// The last composited layer in this layer's child list.
    pub fn last_child(self, app: &App) -> Option<AnyLayer> {
        self.data(app).last_child
    }

    /// Returns whether this layer has at least one child layer.
    pub fn has_children(self, app: &App) -> bool {
        self.data(app).first_child.is_some()
    }

    /// Consider this layer as the root and build a scene (a tree of layers) in the engine.
    // The reason this method is in the `ContainerLayer` class rather than
    // `PipelineOwner` or other singleton level is because this method can be used
    // both to render the whole layer tree (e.g. a normal application frame) and
    // to render a subtree (e.g. `OffsetLayer.toImage`).
    pub fn build_scene(self, app: &mut App, mut builder: SceneBuilder) -> Scene {
        let this = self.as_layer();
        this.update_subtree_needs_add_to_scene(app);
        this.add_to_scene(app, &mut builder);
        if this.subtree_has_composition_callbacks(app) {
            this.fire_composition_callbacks(app, true);
        }
        // Clearing the flag after `add_to_scene`, not before: it calls the children's, which
        // may mark this layer.
        this.data_mut(app).needs_add_to_scene = false;
        builder.build()
    }

    fn debug_ultimate_previous_sibling_of(
        self,
        app: &App,
        mut child: AnyLayer,
        equals: Option<AnyLayer>,
    ) -> bool {
        let attached = self.as_layer().attached(app);
        debug_assert!(child.attached(app) == attached);
        while let Some(previous) = child.previous_sibling(app) {
            debug_assert!(previous != child);
            child = previous;
            debug_assert!(child.attached(app) == attached);
        }
        Some(child) == equals
    }

    fn debug_ultimate_next_sibling_of(
        self,
        app: &App,
        mut child: AnyLayer,
        equals: Option<AnyLayer>,
    ) -> bool {
        let attached = self.as_layer().attached(app);
        debug_assert!(child.attached(app) == attached);
        while let Some(next) = child.next_sibling(app) {
            debug_assert!(next != child);
            child = next;
            debug_assert!(child.attached(app) == attached);
        }
        Some(child) == equals
    }

    /// Adds the given layer to the end of this layer's child list.
    pub fn append(self, app: &mut App, child: AnyLayer) {
        let this = self.as_layer();
        debug_assert!(!this.data(app).debug_mutations_locked);
        debug_assert!(child != this);
        debug_assert!(Some(child) != self.first_child(app));
        debug_assert!(Some(child) != self.last_child(app));
        debug_assert!(child.parent(app).is_none());
        debug_assert!(!child.attached(app));
        debug_assert!(child.next_sibling(app).is_none());
        debug_assert!(child.previous_sibling(app).is_none());
        debug_assert!(child.data(app).parent_handle.layer().is_none());
        debug_assert!(self.debug_root(app) != child); // indicates we are about to create a cycle
        self.adopt_child(app, child);
        let last_child = self.last_child(app);
        child.data_mut(app).previous_sibling = last_child;
        if let Some(last_child) = last_child {
            last_child.data_mut(app).next_sibling = Some(child);
        }
        let data = self.data_mut(app);
        data.last_child = Some(child);
        data.first_child.get_or_insert(child);
        LayerHandle::set_layer(
            app,
            |app| &mut child.data_mut(app).parent_handle,
            Some(child),
        );
        debug_assert!(child.attached(app) == this.attached(app));
    }

    /// The `assert(() { Layer node = this; while (node.parent != null) … }())` walk of `append`
    /// and `_adoptChild`: the root above this layer.
    fn debug_root(self, app: &App) -> AnyLayer {
        let mut node = self.as_layer();
        while let Some(parent) = node.parent(app) {
            node = parent.as_layer();
        }
        node
    }

    fn adopt_child(self, app: &mut App, child: AnyLayer) {
        let this = self.as_layer();
        debug_assert!(!this.data(app).debug_mutations_locked);
        if !this.always_needs_add_to_scene(app) {
            this.mark_needs_add_to_scene(app);
        }
        let callback_count = child.data(app).composition_callback_count;
        if callback_count != 0 {
            this.update_subtree_composition_observer_count(app, callback_count);
        }
        debug_assert!(child.data(app).parent.is_none());
        debug_assert!(self.debug_root(app) != child); // indicates we are about to create a cycle
        child.data_mut(app).parent = Some(self);
        if let Some(owner) = this.owner(app) {
            child.attach(app, owner);
        }
        self.redepth_child(app, child);
    }

    /// Adjust the depth of the given `child` to be greater than this node's own depth.
    ///
    /// Only call this method from overrides of [`Layer::redepth_children`].
    pub fn redepth_child(self, app: &mut App, child: AnyLayer) {
        let this = self.as_layer();
        debug_assert!(child.owner(app) == this.owner(app));
        let depth = this.depth(app);
        if child.depth(app) <= depth {
            child.data_mut(app).depth = depth + 1;
            child.redepth_children(app);
        }
    }

    // Implementation of [Layer.remove].
    pub(crate) fn remove_child(self, app: &mut App, child: AnyLayer) {
        let this = self.as_layer();
        debug_assert!(child.parent(app) == Some(self));
        debug_assert!(child.attached(app) == this.attached(app));
        debug_assert!(self.debug_ultimate_previous_sibling_of(app, child, self.first_child(app)));
        debug_assert!(self.debug_ultimate_next_sibling_of(app, child, self.last_child(app)));
        debug_assert!(child.data(app).parent_handle.layer().is_some());
        let previous_sibling = child.previous_sibling(app);
        let next_sibling = child.next_sibling(app);
        match previous_sibling {
            None => {
                debug_assert!(self.first_child(app) == Some(child));
                self.data_mut(app).first_child = next_sibling;
            }
            Some(previous) => previous.data_mut(app).next_sibling = next_sibling,
        }
        match next_sibling {
            None => {
                debug_assert!(self.last_child(app) == Some(child));
                self.data_mut(app).last_child = previous_sibling;
            }
            Some(next) => next.data_mut(app).previous_sibling = previous_sibling,
        }
        self.debug_check_child_list(app);
        let data = child.data_mut(app);
        data.previous_sibling = None;
        data.next_sibling = None;
        self.drop_child(app, child);
        LayerHandle::set_layer(app, |app| &mut child.data_mut(app).parent_handle, None);
        // The release may have disposed the child; a disposed layer is detached.
        debug_assert!(child.debug_disposed(app) || !child.attached(app));
    }

    /// The asserts of `_removeChild` after the unlink: the list is consistent end to end.
    fn debug_check_child_list(self, app: &App) {
        let attached = self.as_layer().attached(app);
        let first_child = self.first_child(app);
        let last_child = self.last_child(app);
        debug_assert!(first_child.is_none() == last_child.is_none());
        debug_assert!(first_child.is_none_or(|first| first.attached(app) == attached));
        debug_assert!(last_child.is_none_or(|last| last.attached(app) == attached));
        debug_assert!(
            first_child
                .is_none_or(|first| self.debug_ultimate_next_sibling_of(app, first, last_child))
        );
        debug_assert!(
            last_child.is_none_or(|last| self.debug_ultimate_previous_sibling_of(
                app,
                last,
                first_child
            ))
        );
    }

    fn drop_child(self, app: &mut App, child: AnyLayer) {
        let this = self.as_layer();
        debug_assert!(!this.data(app).debug_mutations_locked);
        if !this.always_needs_add_to_scene(app) {
            this.mark_needs_add_to_scene(app);
        }
        let callback_count = child.data(app).composition_callback_count;
        if callback_count != 0 {
            this.update_subtree_composition_observer_count(app, -callback_count);
        }
        debug_assert!(child.data(app).parent == Some(self));
        debug_assert!(child.attached(app) == this.attached(app));
        child.data_mut(app).parent = None;
        if this.attached(app) {
            child.detach(app);
        }
    }

    /// Removes all of this layer's children from its child list.
    pub fn remove_all_children(self, app: &mut App) {
        let this = self.as_layer();
        debug_assert!(!this.data(app).debug_mutations_locked);
        let mut child = self.first_child(app);
        while let Some(layer) = child {
            let next = layer.next_sibling(app);
            let data = layer.data_mut(app);
            data.previous_sibling = None;
            data.next_sibling = None;
            debug_assert!(layer.attached(app) == this.attached(app));
            self.drop_child(app, layer);
            LayerHandle::set_layer(app, |app| &mut layer.data_mut(app).parent_handle, None);
            child = next;
        }
        let data = self.data_mut(app);
        data.first_child = None;
        data.last_child = None;
    }

    /// Uploads all of this layer's children to the engine.
    ///
    /// This method is typically used by `add_to_scene` to insert the children into the scene.
    /// Subclasses of [`ContainerLayer`] typically override `add_to_scene` to apply effects to
    /// the scene using the [`SceneBuilder`] API, then insert their children using
    /// `add_children_to_scene`, then reverse the aforementioned effects before returning from
    /// `add_to_scene`.
    pub fn add_children_to_scene(self, app: &mut App, builder: &mut SceneBuilder) {
        let mut child = self.first_child(app);
        while let Some(layer) = child {
            layer.add_to_scene_with_retained_rendering(app, builder);
            child = layer.next_sibling(app);
        }
    }

    /// See [`ContainerLayer::apply_transform`].
    pub fn apply_transform(self, app: &App, child: Option<AnyLayer>, transform: &mut Matrix4) {
        (self.vtable.apply_transform)(app, self.id, child, transform);
    }

    /// Returns the descendants of this layer in depth first order.
    pub fn depth_first_iterate_children(self, app: &App) -> Vec<AnyLayer> {
        let mut children = Vec::new();
        let mut child = self.first_child(app);
        while let Some(layer) = child {
            children.push(layer);
            if let Some(container) = layer.as_container_layer() {
                children.extend(container.depth_first_iterate_children(app));
            }
            child = layer.next_sibling(app);
        }
        children
    }
}

/// Flutter's `OffsetLayer` fields, held under the field `offset_layer`
/// ([`offset_layer_accessors!`]).
#[derive(Debug)]
pub struct OffsetLayerData {
    offset: Offset,
}

impl OffsetLayerData {
    /// Flutter's `OffsetLayer({offset})`.
    pub const fn new(offset: Offset) -> OffsetLayerData {
        OffsetLayerData { offset }
    }
}

impl Default for OffsetLayerData {
    /// Flutter's `OffsetLayer()`: `offset` is zero.
    fn default() -> OffsetLayerData {
        OffsetLayerData::new(Offset::ZERO)
    }
}

/// Implements the [`OffsetLayerMixin`] field accessors for an `offset_layer` field.
#[macro_export]
macro_rules! offset_layer_accessors {
    () => {
        fn offset_layer_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::OffsetLayerData {
            &app.get(self).offset_layer
        }
        fn offset_layer_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::OffsetLayerData {
            &mut app.get_mut(self).offset_layer
        }
    };
}

/// A layer that is displayed at an offset from its parent layer.
///
/// Offset layers are key to efficient repainting because they are created by repaint boundaries
/// in the `RenderObject` tree (see `RenderObject.isRepaintBoundary`). When a render object that
/// is a repaint boundary is asked to paint at given offset in a `PaintingContext`, the render
/// object first checks whether it needs to repaint itself. If not, it reuses its existing
/// [`OffsetLayer`] (and its entire subtree) by mutating its offset property, cutting off the
/// paint walk.
///
/// Flutter's `OffsetLayer` is a concrete class that `TransformLayer`, `OpacityLayer` and
/// `ImageFilterLayer` extend: the bodies are here, [`OffsetLayer`] is the concrete leaf.
pub trait OffsetLayerMixin: ContainerLayer {
    /// Mixin field access.
    fn offset_layer_data(self: Handle<Self>, app: &App) -> &OffsetLayerData;

    /// See [`offset_layer_data`](Self::offset_layer_data).
    fn offset_layer_data_mut(self: Handle<Self>, app: &mut App) -> &mut OffsetLayerData;

    /// Dart's implicit upcast of a concrete layer to `OffsetLayer`.
    fn as_offset_layer(self: Handle<Self>) -> AnyOffsetLayer {
        AnyOffsetLayer {
            id: self.id(),
            vtable: const { &OffsetLayerVTable::of::<Self>() },
        }
    }

    /// Offset from parent in the parent's coordinate system.
    fn offset(self: Handle<Self>, app: &App) -> Offset {
        self.offset_layer_data(app).offset
    }

    /// See [`offset`](Self::offset).
    fn set_offset(self: Handle<Self>, app: &mut App, value: Offset) {
        self.offset_layer_data_mut(app).offset = value;
    }

    /// `OffsetLayer.findAnnotations`.
    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let offset = self.offset(app);
        ContainerLayer::find_annotations(self, app, result, local_position - offset, only_first)
    }

    /// `OffsetLayer.applyTransform`.
    fn apply_transform(
        self: Handle<Self>,
        app: &App,
        child: Option<AnyLayer>,
        transform: &mut Matrix4,
    ) {
        debug_assert!(child.is_some());
        let offset = self.offset(app);
        *transform = transform.then(&Matrix4::translation(
            offset.dx() as f32,
            offset.dy() as f32,
        ));
    }

    /// `OffsetLayer.addToScene`.
    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        // Skia has a fast path for concatenating scale/translation only matrices.
        // Hence pushing a translation-only transform layer should be fast. For
        // retained rendering, we don't want to push the offset down to each leaf
        // node. Otherwise, changing an offset layer on the very high level could
        // cascade the change to too many leaves.
        let offset = self.offset(app);
        builder.push_offset(offset.dx(), offset.dy());
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = builder.pop();
        self.as_layer().set_engine_layer(app, Some(engine_layer));
    }
}

/// The vtable of an [`AnyOffsetLayer`]: the container vtable plus the offset accessors.
///
/// Public only so [`container_layer_accessors!`] can name it from another crate.
#[doc(hidden)]
pub struct OffsetLayerVTable {
    container: ContainerLayerVTable,
    offset_data: fn(&App, HandleId) -> &OffsetLayerData,
    offset_data_mut: fn(&mut App, HandleId) -> &mut OffsetLayerData,
}

impl OffsetLayerVTable {
    pub const fn of<T: OffsetLayerMixin>() -> OffsetLayerVTable {
        OffsetLayerVTable {
            container: ContainerLayerVTable::of::<T>(),
            offset_data: |app, id| T::offset_layer_data(resolve(id), app),
            offset_data_mut: |app, id| T::offset_layer_data_mut(resolve(id), app),
        }
    }
}

/// Erased `OffsetLayer`: what `RenderObject.updateCompositedLayer` returns and
/// `PaintingContext` positions.
#[derive(Clone, Copy)]
pub struct AnyOffsetLayer {
    id: HandleId,
    vtable: &'static OffsetLayerVTable,
}

impl PartialEq for AnyOffsetLayer {
    fn eq(&self, other: &AnyOffsetLayer) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyOffsetLayer {}

impl Hash for AnyOffsetLayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<AnyOffsetLayer> for HandleId {
    fn from(layer: AnyOffsetLayer) -> HandleId {
        layer.id
    }
}

impl Debug for AnyOffsetLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyOffsetLayer({:?})", self.id)
    }
}

impl ErasedLayer for AnyOffsetLayer {
    fn as_layer(self) -> AnyLayer {
        self.as_container_layer().as_layer()
    }
}

impl AnyOffsetLayer {
    fn from_vtable(id: HandleId, vtable: &'static OffsetLayerVTable) -> AnyOffsetLayer {
        AnyOffsetLayer { id, vtable }
    }

    pub fn id(self) -> HandleId {
        self.id
    }

    /// The `ContainerLayer` view of this layer. Free: points into the nested table.
    pub fn as_container_layer(self) -> AnyContainerLayer {
        AnyContainerLayer::from_vtable(self.id, &self.vtable.container)
    }

    /// See [`OffsetLayerMixin::offset`].
    pub fn offset(self, app: &App) -> Offset {
        (self.vtable.offset_data)(app, self.id).offset
    }

    /// See [`OffsetLayerMixin::set_offset`].
    pub fn set_offset(self, app: &mut App, value: Offset) {
        if value == self.offset(app) {
            return;
        }
        (self.vtable.offset_data_mut)(app, self.id).offset = value;
        if !self.as_layer().always_needs_add_to_scene(app) {
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

/// Flutter's concrete `OffsetLayer`; the bodies are [`OffsetLayerMixin`].
pub struct OffsetLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    offset_layer: OffsetLayerData,
}

impl OffsetLayer {
    /// Creates an offset layer.
    ///
    /// By default, `offset` is zero. It must be set before the compositing phase of the pipeline.
    pub fn new(app: &mut App, offset: Offset) -> Handle<OffsetLayer> {
        app.create(OffsetLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            offset_layer: OffsetLayerData::new(offset),
        })
    }
}

impl Layer for OffsetLayer {
    crate::layer_accessors!(container);

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        OffsetLayerMixin::find_annotations(self, app, result, local_position, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        OffsetLayerMixin::add_to_scene(self, app, builder);
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for OffsetLayer {
    crate::container_layer_accessors!(offset);

    fn apply_transform(
        self: Handle<Self>,
        app: &App,
        child: Option<AnyLayer>,
        transform: &mut Matrix4,
    ) {
        OffsetLayerMixin::apply_transform(self, app, child, transform);
    }
}

impl OffsetLayerMixin for OffsetLayer {
    crate::offset_layer_accessors!();
}

/// A key that identifies a group of backdrop filters that share one backdrop.
///
/// Every `BackdropFilterLayer` with the same key blurs the scene once, as of the first of
/// them, which is what Flutter's `BackdropGroup` trades for the cost of one filter pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BackdropKey(u64);

static NEXT_BACKDROP_KEY: AtomicU64 = AtomicU64::new(0);

impl BackdropKey {
    /// Create a new [`BackdropKey`].
    #[expect(
        clippy::new_without_default,
        reason = "each call creates a distinct key; Default would suggest a neutral value"
    )]
    pub fn new() -> BackdropKey {
        BackdropKey(NEXT_BACKDROP_KEY.fetch_add(1, Ordering::Relaxed))
    }

    /// dart:ui's `backdropId` for [`SceneBuilder::push_backdrop_filter`].
    pub fn id(self) -> u64 {
        self.0
    }
}

/// A composite layer that clips its children using a rectangle.
///
/// When debugging, setting [`debug_disable_clip_layers`](crate::debug::debug_disable_clip_layers)
/// to true will cause this layer to be skipped (directly replaced by its children). This can be
/// helpful to track down the cause of performance problems.
pub struct ClipRectLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    clip_rect: Option<Rect>,
    clip_behavior: Clip,
}

impl ClipRectLayer {
    /// Creates a layer with a rectangular clip.
    ///
    /// The [`clip_rect`](Self::clip_rect) argument must not be null before the compositing phase
    /// of the pipeline.
    ///
    /// The [`clip_behavior`](Self::clip_behavior) argument must not be [`Clip::None`].
    pub fn new(app: &mut App) -> Handle<ClipRectLayer> {
        debug_assert!(Clip::HardEdge != Clip::None);
        app.create(ClipRectLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            clip_rect: None,
            clip_behavior: Clip::HardEdge,
        })
    }

    /// The rectangle to clip in the parent's coordinate system.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    pub fn clip_rect(self: Handle<Self>, app: &App) -> Option<Rect> {
        app.get(self).clip_rect
    }

    /// See [`clip_rect`](Self::clip_rect).
    pub fn set_clip_rect(self: Handle<Self>, app: &mut App, value: Option<Rect>) {
        if value != app.get(self).clip_rect {
            app.get_mut(self).clip_rect = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }

    /// Controls how to clip.
    ///
    /// Must not be set to null or [`Clip::None`].
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub fn clip_behavior(self: Handle<Self>, app: &App) -> Clip {
        app.get(self).clip_behavior
    }

    /// See [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: Handle<Self>, app: &mut App, value: Clip) {
        debug_assert!(value != Clip::None);
        if value != app.get(self).clip_behavior {
            app.get_mut(self).clip_behavior = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

impl Layer for ClipRectLayer {
    crate::layer_accessors!(container);

    fn describe_clip_bounds(self: Handle<Self>, app: &App) -> Option<Rect> {
        self.clip_rect(app)
    }

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        if !self
            .clip_rect(app)
            .expect("clipRect must be set before searching annotations")
            .contains(local_position)
        {
            return false;
        }
        ContainerLayer::find_annotations(self, app, result, local_position, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        debug_assert!(self.clip_rect(app).is_some());
        let mut enabled = true;
        if cfg!(debug_assertions) {
            enabled = !crate::debug::debug_disable_clip_layers();
        }
        if enabled {
            let clip_rect = self
                .clip_rect(app)
                .expect("clipRect must be set before compositing");
            let clip_behavior = self.clip_behavior(app);
            builder.push_clip_rect(clip_rect, clip_behavior);
        }
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = enabled.then(|| builder.pop());
        self.as_layer().set_engine_layer(app, engine_layer);
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for ClipRectLayer {
    crate::container_layer_accessors!();
}

/// A composite layer that clips its children using a rounded rectangle.
///
/// When debugging, setting [`debug_disable_clip_layers`](crate::debug::debug_disable_clip_layers)
/// to true will cause this layer to be skipped (directly replaced by its children). This can be
/// helpful to track down the cause of performance problems.
pub struct ClipRRectLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    clip_rrect: Option<RRect>,
    clip_behavior: Clip,
}

impl ClipRRectLayer {
    /// Creates a layer with a rounded-rectangular clip.
    ///
    /// The [`clip_rrect`](Self::clip_rrect) and [`clip_behavior`](Self::clip_behavior) properties
    /// must be non-null before the compositing phase of the pipeline.
    pub fn new(app: &mut App) -> Handle<ClipRRectLayer> {
        debug_assert!(Clip::AntiAlias != Clip::None);
        app.create(ClipRRectLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            clip_rrect: None,
            clip_behavior: Clip::AntiAlias,
        })
    }

    /// The rounded-rect to clip in the parent's coordinate system.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    pub fn clip_rrect(self: Handle<Self>, app: &App) -> Option<RRect> {
        app.get(self).clip_rrect
    }

    /// See [`clip_rrect`](Self::clip_rrect).
    pub fn set_clip_rrect(self: Handle<Self>, app: &mut App, value: Option<RRect>) {
        if value != app.get(self).clip_rrect {
            app.get_mut(self).clip_rrect = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }

    /// Controls how to clip.
    ///
    /// Must not be set to null or [`Clip::None`].
    ///
    /// Defaults to [`Clip::AntiAlias`].
    pub fn clip_behavior(self: Handle<Self>, app: &App) -> Clip {
        app.get(self).clip_behavior
    }

    /// See [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: Handle<Self>, app: &mut App, value: Clip) {
        debug_assert!(value != Clip::None);
        if value != app.get(self).clip_behavior {
            app.get_mut(self).clip_behavior = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

impl Layer for ClipRRectLayer {
    crate::layer_accessors!(container);

    fn describe_clip_bounds(self: Handle<Self>, app: &App) -> Option<Rect> {
        self.clip_rrect(app).map(RRect::outer_rect)
    }

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        if !self
            .clip_rrect(app)
            .expect("clipRRect must be set before searching annotations")
            .contains(local_position)
        {
            return false;
        }
        ContainerLayer::find_annotations(self, app, result, local_position, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        debug_assert!(self.clip_rrect(app).is_some());
        let mut enabled = true;
        if cfg!(debug_assertions) {
            enabled = !crate::debug::debug_disable_clip_layers();
        }
        if enabled {
            let clip_rrect = self
                .clip_rrect(app)
                .expect("clipRRect must be set before compositing");
            let clip_behavior = self.clip_behavior(app);
            builder.push_clip_rrect(clip_rrect, clip_behavior);
        }
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = enabled.then(|| builder.pop());
        self.as_layer().set_engine_layer(app, engine_layer);
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for ClipRRectLayer {
    crate::container_layer_accessors!();
}

/// A composite layer that clips its children using a path.
///
/// When debugging, setting [`debug_disable_clip_layers`](crate::debug::debug_disable_clip_layers)
/// to true will cause this layer to be skipped (directly replaced by its children). This can be
/// helpful to track down the cause of performance problems.
pub struct ClipPathLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    clip_path: Option<Arc<inset_embedder::Path>>,
    clip_behavior: Clip,
}

impl ClipPathLayer {
    /// Creates a layer with a path-based clip.
    ///
    /// The [`clip_path`](Self::clip_path) and [`clip_behavior`](Self::clip_behavior) properties
    /// must be non-null before the compositing phase of the pipeline.
    pub fn new(app: &mut App) -> Handle<ClipPathLayer> {
        debug_assert!(Clip::AntiAlias != Clip::None);
        app.create(ClipPathLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            clip_path: None,
            clip_behavior: Clip::AntiAlias,
        })
    }

    /// The path to clip in the parent's coordinate system.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    pub fn clip_path(self: Handle<Self>, app: &App) -> Option<&Arc<inset_embedder::Path>> {
        app.get(self).clip_path.as_ref()
    }

    /// See [`clip_path`](Self::clip_path).
    pub fn set_clip_path(
        self: Handle<Self>,
        app: &mut App,
        value: Option<Arc<inset_embedder::Path>>,
    ) {
        app.get_mut(self).clip_path = value;
        self.as_layer().mark_needs_add_to_scene(app);
    }

    /// Controls how to clip.
    ///
    /// Must not be set to null or [`Clip::None`].
    ///
    /// Defaults to [`Clip::AntiAlias`].
    pub fn clip_behavior(self: Handle<Self>, app: &App) -> Clip {
        app.get(self).clip_behavior
    }

    /// See [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: Handle<Self>, app: &mut App, value: Clip) {
        debug_assert!(value != Clip::None);
        if value != app.get(self).clip_behavior {
            app.get_mut(self).clip_behavior = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

impl Layer for ClipPathLayer {
    crate::layer_accessors!(container);

    fn describe_clip_bounds(self: Handle<Self>, app: &App) -> Option<Rect> {
        let bounds = app.get(self).clip_path.as_ref()?.bounds();
        Some(Rect::from_ltwh(
            bounds.x as f64,
            bounds.y as f64,
            bounds.width as f64,
            bounds.height as f64,
        ))
    }

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let path = app
            .get(self)
            .clip_path
            .as_ref()
            .expect("clipPath must be set before searching annotations");
        if !path.contains(local_position.into(), FillRule::NonZero) {
            return false;
        }
        ContainerLayer::find_annotations(self, app, result, local_position, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        debug_assert!(app.get(self).clip_path.is_some());
        let mut enabled = true;
        if cfg!(debug_assertions) {
            enabled = !crate::debug::debug_disable_clip_layers();
        }
        if enabled {
            let clip_path = Arc::clone(
                app.get(self)
                    .clip_path
                    .as_ref()
                    .expect("clipPath must be set before compositing"),
            );
            let clip_behavior = self.clip_behavior(app);
            builder.push_clip_path(&clip_path, clip_behavior);
        }
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = enabled.then(|| builder.pop());
        self.as_layer().set_engine_layer(app, engine_layer);
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for ClipPathLayer {
    crate::container_layer_accessors!();
}

/// Dart's `PointerEvent.removePerspectiveTransform`: the third row and column become
/// `(0, 0, 1, 0)` so a transformed point stays under the pointer.
fn remove_perspective_transform(transform: Matrix4) -> Matrix4 {
    let mut storage = transform.to_flutter_array();
    storage[8] = 0.0;
    storage[9] = 0.0;
    storage[10] = 1.0;
    storage[11] = 0.0;
    storage[2] = 0.0;
    storage[6] = 0.0;
    storage[10] = 1.0;
    storage[14] = 0.0;
    Matrix4::from_flutter_array(&storage)
}

/// Dart's `MatrixUtils.transformPoint`.
fn transform_point(transform: &Matrix4, point: Offset) -> Offset {
    let storage = transform.to_flutter_array();
    let x = point.dx();
    let y = point.dy();
    let rx = storage[0] as f64 * x + storage[4] as f64 * y + storage[12] as f64;
    let ry = storage[1] as f64 * x + storage[5] as f64 * y + storage[13] as f64;
    let rw = storage[3] as f64 * x + storage[7] as f64 * y + storage[15] as f64;
    if rw == 1.0 {
        Offset::new(rx, ry)
    } else {
        Offset::new(rx / rw, ry / rw)
    }
}

/// A composited layer that applies a given transformation matrix to its children.
///
/// This class inherits from [`OffsetLayer`] to make it one of the layers that can be used at the
/// root of a `RenderObject` hierarchy.
pub struct TransformLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    offset_layer: OffsetLayerData,
    transform: Option<Matrix4>,
    last_effective_transform: Option<Matrix4>,
    inverted_transform: Cell<Option<Matrix4>>,
    inverse_dirty: Cell<bool>,
}

impl TransformLayer {
    /// Creates a transform layer.
    ///
    /// The [`transform`](Self::transform) and `offset` properties must be non-null before the
    /// compositing phase of the pipeline.
    pub fn new(app: &mut App) -> Handle<TransformLayer> {
        app.create(TransformLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            offset_layer: OffsetLayerData::new(Offset::ZERO),
            transform: None,
            last_effective_transform: None,
            inverted_transform: Cell::new(None),
            inverse_dirty: Cell::new(true),
        })
    }

    /// The matrix to apply.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    ///
    /// This transform is applied before `offset`, if both are set.
    ///
    /// The [`transform`](Self::transform) property must be non-null before the compositing phase
    /// of the pipeline.
    pub fn transform(self: Handle<Self>, app: &App) -> Option<Matrix4> {
        app.get(self).transform
    }

    /// See [`transform`](Self::transform).
    pub fn set_transform(self: Handle<Self>, app: &mut App, value: Matrix4) {
        debug_assert!(
            value
                .to_flutter_array()
                .iter()
                .all(|component| component.is_finite())
        );
        if app.get(self).transform == Some(value) {
            return;
        }
        app.get_mut(self).transform = Some(value);
        app.get(self).inverse_dirty.set(true);
        self.as_layer().mark_needs_add_to_scene(app);
    }

    fn transform_offset(self: Handle<Self>, app: &App, local_position: Offset) -> Option<Offset> {
        if app.get(self).inverse_dirty.get() {
            let transform = app
                .get(self)
                .transform
                .expect("transform must be set before searching annotations");
            app.get(self)
                .inverted_transform
                .set(remove_perspective_transform(transform).invert());
            app.get(self).inverse_dirty.set(false);
        }
        app.get(self)
            .inverted_transform
            .get()
            .map(|inverted| transform_point(&inverted, local_position))
    }
}

impl Layer for TransformLayer {
    crate::layer_accessors!(container);

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let Some(transformed_offset) = self.transform_offset(app, local_position) else {
            return false;
        };
        OffsetLayerMixin::find_annotations(self, app, result, transformed_offset, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        debug_assert!(self.transform(app).is_some());
        let mut last_effective = self
            .transform(app)
            .expect("transform must be set before compositing");
        let offset = self.offset(app);
        if offset != Offset::ZERO {
            last_effective =
                Matrix4::translation(offset.dx() as f32, offset.dy() as f32).then(&last_effective);
        }
        app.get_mut(self).last_effective_transform = Some(last_effective);
        builder.push_transform(&last_effective);
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = builder.pop();
        self.as_layer().set_engine_layer(app, Some(engine_layer));
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for TransformLayer {
    crate::container_layer_accessors!(offset);

    fn apply_transform(
        self: Handle<Self>,
        app: &App,
        child: Option<AnyLayer>,
        transform: &mut Matrix4,
    ) {
        debug_assert!(child.is_some());
        debug_assert!(
            app.get(self).last_effective_transform.is_some() || self.transform(app).is_some()
        );
        let applied = match app.get(self).last_effective_transform {
            None => self
                .transform(app)
                .expect("transform must be set before applyTransform"),
            Some(last_effective) => last_effective,
        };
        *transform = transform.then(&applied);
    }
}

impl OffsetLayerMixin for TransformLayer {
    crate::offset_layer_accessors!();
}

/// A composited layer that makes its children partially transparent.
///
/// When debugging, setting
/// [`debug_disable_opacity_layers`](crate::debug::debug_disable_opacity_layers) to true will
/// cause this layer to be skipped (directly replaced by its children). This can be helpful to
/// track down the cause of performance problems.
///
/// Try to avoid an [`OpacityLayer`] with no children. Remove that layer if possible to save some
/// tree walks.
pub struct OpacityLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    offset_layer: OffsetLayerData,
    alpha: Option<i32>,
}

impl OpacityLayer {
    /// Creates an opacity layer.
    ///
    /// The [`alpha`](Self::alpha) property must be non-null before the compositing phase of the
    /// pipeline.
    pub fn new(app: &mut App) -> Handle<OpacityLayer> {
        app.create(OpacityLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            offset_layer: OffsetLayerData::new(Offset::ZERO),
            alpha: None,
        })
    }

    /// The amount to multiply into the alpha channel.
    ///
    /// The opacity is expressed as an integer from 0 to 255, where 0 is fully transparent and
    /// 255 is fully opaque.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    pub fn alpha(self: Handle<Self>, app: &App) -> Option<i32> {
        app.get(self).alpha
    }

    /// See [`alpha`](Self::alpha).
    pub fn set_alpha(self: Handle<Self>, app: &mut App, value: i32) {
        if app.get(self).alpha != Some(value) {
            app.get_mut(self).alpha = Some(value);
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

impl Layer for OpacityLayer {
    crate::layer_accessors!(container);

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        OffsetLayerMixin::find_annotations(self, app, result, local_position, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        debug_assert!(self.alpha(app).is_some());

        // Don't add this layer if there's no child.
        let mut enabled = self.as_container_layer().first_child(app).is_some();
        if !enabled {
            // Ensure the engine layer is disposed.
            self.as_layer().set_engine_layer(app, None);
            return;
        }

        if cfg!(debug_assertions) {
            enabled = enabled && !crate::debug::debug_disable_opacity_layers();
        }

        let realized_alpha = self
            .alpha(app)
            .expect("alpha must be set before compositing");
        let offset = self.offset(app);
        if enabled && realized_alpha < 255 {
            builder.push_opacity(realized_alpha, offset);
        } else {
            builder.push_offset(offset.dx(), offset.dy());
        }
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = builder.pop();
        self.as_layer().set_engine_layer(app, Some(engine_layer));
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for OpacityLayer {
    crate::container_layer_accessors!(offset);

    fn apply_transform(
        self: Handle<Self>,
        app: &App,
        child: Option<AnyLayer>,
        transform: &mut Matrix4,
    ) {
        OffsetLayerMixin::apply_transform(self, app, child, transform);
    }
}

impl OffsetLayerMixin for OpacityLayer {
    crate::offset_layer_accessors!();
}

/// A composited layer that applies a filter to the existing contents of the scene.
pub struct BackdropFilterLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    filter: Option<ImageFilter>,
    blend_mode: BlendMode,
    backdrop_key: Option<BackdropKey>,
}

impl BackdropFilterLayer {
    /// Creates a backdrop filter layer.
    ///
    /// The [`filter`](Self::filter) property must be non-null before the compositing phase of
    /// the pipeline.
    ///
    /// The [`blend_mode`](Self::blend_mode) property defaults to [`BlendMode::SrcOver`].
    pub fn new(app: &mut App) -> Handle<BackdropFilterLayer> {
        app.create(BackdropFilterLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            filter: None,
            blend_mode: BlendMode::SrcOver,
            backdrop_key: None,
        })
    }

    /// The filter to apply to the existing contents of the scene.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    pub fn filter(self: Handle<Self>, app: &App) -> Option<&ImageFilter> {
        app.get(self).filter.as_ref()
    }

    /// See [`filter`](Self::filter).
    pub fn set_filter(self: Handle<Self>, app: &mut App, value: Option<ImageFilter>) {
        app.get_mut(self).filter = value;
        self.as_layer().mark_needs_add_to_scene(app);
    }

    /// The blend mode to use to apply the filtered background content onto the background
    /// surface.
    ///
    /// The default value of this property is [`BlendMode::SrcOver`].
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    pub fn blend_mode(self: Handle<Self>, app: &App) -> BlendMode {
        app.get(self).blend_mode
    }

    /// See [`blend_mode`](Self::blend_mode).
    pub fn set_blend_mode(self: Handle<Self>, app: &mut App, value: BlendMode) {
        if value != app.get(self).blend_mode {
            app.get_mut(self).blend_mode = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }

    /// The backdrop key that identifies the `BackdropGroup` this filter will apply to.
    ///
    /// The default value for the backdrop key is `None`, meaning that it's not part of a
    /// `BackdropGroup`.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    pub fn backdrop_key(self: Handle<Self>, app: &App) -> Option<BackdropKey> {
        app.get(self).backdrop_key
    }

    /// See [`backdrop_key`](Self::backdrop_key).
    pub fn set_backdrop_key(self: Handle<Self>, app: &mut App, value: Option<BackdropKey>) {
        if value != app.get(self).backdrop_key {
            app.get_mut(self).backdrop_key = value;
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

impl Layer for BackdropFilterLayer {
    crate::layer_accessors!(container);

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        ContainerLayer::find_annotations(self, app, result, local_position, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        debug_assert!(app.get(self).filter.is_some());
        let filter = app
            .get(self)
            .filter
            .clone()
            .expect("filter must be set before compositing");
        let blend_mode = self.blend_mode(app);
        let backdrop_id = self.backdrop_key(app).map(BackdropKey::id);
        builder.push_backdrop_filter(&filter, blend_mode, backdrop_id);
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = builder.pop();
        self.as_layer().set_engine_layer(app, Some(engine_layer));
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for BackdropFilterLayer {
    crate::container_layer_accessors!();
}

/// An object that a [`LeaderLayer`] can register with.
///
/// An instance of this class should be provided as the [`LeaderLayer::link`] and the
/// [`FollowerLayer::link`] properties to cause the [`FollowerLayer`] to follow the
/// [`LeaderLayer`].
///
/// See also:
///
///  * `CompositedTransformTarget`, the widget that creates a [`LeaderLayer`].
///  * `CompositedTransformFollower`, the widget that creates a [`FollowerLayer`].
///  * `RenderLeaderLayer` and `RenderFollowerLayer`, the corresponding render objects.
pub struct LayerLink {
    leader: Option<Handle<LeaderLayer>>,
    debug_previous_leaders: Option<HashSet<Handle<LeaderLayer>>>,
    debug_leader_check_scheduled: bool,
    /// The total size of the content of the connected [`LeaderLayer`].
    ///
    /// Generally this should be set by the `RenderObject` that paints on the registered
    /// [`LeaderLayer`] (for instance a `RenderLeaderLayer` that shares this link with its
    /// followers). This size may be outdated before and during layout.
    pub leader_size: Option<Size>,
}

impl LayerLink {
    /// Creates a link with no leader.
    pub fn new(app: &mut App) -> Handle<LayerLink> {
        app.create(LayerLink {
            leader: None,
            debug_previous_leaders: None,
            debug_leader_check_scheduled: false,
            leader_size: None,
        })
    }

    /// The [`LeaderLayer`] connected to this link.
    pub fn leader(self: Handle<Self>, app: &App) -> Option<Handle<LeaderLayer>> {
        app.get(self).leader
    }

    fn register_leader(self: Handle<Self>, app: &mut App, leader: Handle<LeaderLayer>) {
        debug_assert_ne!(self.leader(app), Some(leader));
        if cfg!(debug_assertions)
            && let Some(current) = self.leader(app)
        {
            let previous = app
                .get_mut(self)
                .debug_previous_leaders
                .get_or_insert_with(HashSet::new);
            debug_assert!(previous.insert(current));
            self.debug_schedule_leaders_clean_up_check(app);
        }
        app.get_mut(self).leader = Some(leader);
    }

    fn unregister_leader(self: Handle<Self>, app: &mut App, leader: Handle<LeaderLayer>) {
        if self.leader(app) == Some(leader) {
            app.get_mut(self).leader = None;
        } else if cfg!(debug_assertions) {
            let removed = app
                .get_mut(self)
                .debug_previous_leaders
                .as_mut()
                .expect("a replaced leader is tracked")
                .remove(&leader);
            debug_assert!(removed);
        }
    }

    /// Schedules the check as post frame callback to make sure the `_debugPreviousLeaders` is
    /// empty.
    fn debug_schedule_leaders_clean_up_check(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).debug_previous_leaders.is_some());
        if cfg!(debug_assertions) {
            if app.get(self).debug_leader_check_scheduled {
                return;
            }
            app.get_mut(self).debug_leader_check_scheduled = true;
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(self, |this, app, _time_stamp| {
                    app.get_mut(this).debug_leader_check_scheduled = false;
                    debug_assert!(
                        app.get(this)
                            .debug_previous_leaders
                            .as_ref()
                            .expect("a replaced leader is tracked")
                            .is_empty()
                    );
                }),
            );
        }
    }
}

/// A composited layer that can be followed by a [`FollowerLayer`].
///
/// This layer collapses the accumulated offset into a transform and passes [`Offset::ZERO`] to
/// its child layers in the [`Layer::add_to_scene`] / `add_children_to_scene` methods, so that
/// [`ContainerLayer::apply_transform`] will work reliably.
pub struct LeaderLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    /// `None` only while [`Layer::dispose`] takes the retain so it can [`App::release`] it
    /// before the arena entry is destroyed.
    link: Option<RetainedHandle<Handle<LayerLink>>>,
    offset: Offset,
}

impl LeaderLayer {
    /// Creates a leader layer.
    ///
    /// The [`link`](Self::link) property must not have been provided to any other [`LeaderLayer`]
    /// layers that are attached to the layer tree at the same time.
    ///
    /// The [`offset`](Self::offset) property must be non-null before the compositing phase of
    /// the pipeline.
    pub fn new(app: &mut App, link: Handle<LayerLink>) -> Handle<LeaderLayer> {
        let link = app.retain(link);
        app.create(LeaderLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            link: Some(link),
            offset: Offset::ZERO,
        })
    }

    /// The object with which this layer should register.
    ///
    /// The link will be established when this layer is [`Layer::attach`]ed, and will be cleared
    /// when this layer is [`Layer::detach`]ed.
    pub fn link(self: Handle<Self>, app: &App) -> Handle<LayerLink> {
        app.get(self)
            .link
            .as_ref()
            .expect("a LeaderLayer holds its link until dispose")
            .get()
    }

    /// See [`link`](Self::link).
    pub fn set_link(self: Handle<Self>, app: &mut App, value: Handle<LayerLink>) {
        let current = self.link(app);
        if current == value {
            return;
        }
        if self.as_layer().attached(app) {
            current.unregister_leader(app, self);
            value.register_leader(app, self);
        }
        let retained = app.retain(value);
        let old = app
            .get_mut(self)
            .link
            .replace(retained)
            .expect("a LeaderLayer holds its link until dispose");
        app.release(old);
    }

    /// Offset from parent in the parent's coordinate system.
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    ///
    /// The [`offset`](Self::offset) property must be non-null before the compositing phase of
    /// the pipeline.
    pub fn offset(self: Handle<Self>, app: &App) -> Offset {
        app.get(self).offset
    }

    /// See [`offset`](Self::offset).
    pub fn set_offset(self: Handle<Self>, app: &mut App, value: Offset) {
        if value == app.get(self).offset {
            return;
        }
        app.get_mut(self).offset = value;
        if !self.as_layer().always_needs_add_to_scene(app) {
            self.as_layer().mark_needs_add_to_scene(app);
        }
    }
}

impl Layer for LeaderLayer {
    crate::layer_accessors!(container);

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let link = app
            .get_mut(self)
            .link
            .take()
            .expect("a LeaderLayer holds its link until dispose");
        ContainerLayer::dispose(self, app);
        app.release(link);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
        let link = self.link(app);
        link.register_leader(app, self);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        let link = self.link(app);
        link.unregister_leader(app, self);
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let offset = self.offset(app);
        ContainerLayer::find_annotations(self, app, result, local_position - offset, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        let offset = self.offset(app);
        if offset != Offset::ZERO {
            builder.push_transform(&Matrix4::translation(
                offset.dx() as f32,
                offset.dy() as f32,
            ));
        }
        self.as_container_layer()
            .add_children_to_scene(app, builder);
        let engine_layer = (offset != Offset::ZERO).then(|| builder.pop());
        self.as_layer().set_engine_layer(app, engine_layer);
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for LeaderLayer {
    crate::container_layer_accessors!();

    /// Applies the transform that would be applied when compositing the given child to the given
    /// matrix.
    ///
    /// See [`ContainerLayer::apply_transform`] for details.
    ///
    /// The `child` argument may be null, as the same transform is applied to all children.
    fn apply_transform(
        self: Handle<Self>,
        app: &App,
        _child: Option<AnyLayer>,
        transform: &mut Matrix4,
    ) {
        let offset = self.offset(app);
        if offset != Offset::ZERO {
            *transform = transform.then(&Matrix4::translation(
                offset.dx() as f32,
                offset.dy() as f32,
            ));
        }
    }
}

/// A composited layer that applies a transformation matrix to its children such that they are
/// positioned to match a [`LeaderLayer`].
///
/// If any of the ancestors of this layer have a degenerate matrix (e.g. scaling by zero), then
/// the [`FollowerLayer`] will not be able to transform its child to the coordinate space of the
/// [`LeaderLayer`].
///
/// A [`linked_offset`](FollowerLayer::linked_offset) property can be provided to further offset
/// the child layer from the leader layer, for example if the child is to follow the linked layer
/// at a distance rather than directly overlapping it.
pub struct FollowerLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    /// `None` only while [`Layer::dispose`] takes the retain so it can [`App::release`] it
    /// before the arena entry is destroyed.
    link: Option<RetainedHandle<Handle<LayerLink>>>,
    show_when_unlinked: bool,
    unlinked_offset: Offset,
    linked_offset: Offset,
    last_offset: Option<Offset>,
    last_transform: Option<Matrix4>,
    inverted_transform: Cell<Option<Matrix4>>,
    inverse_dirty: Cell<bool>,
}

impl FollowerLayer {
    /// Creates a follower layer.
    ///
    /// The [`unlinked_offset`](Self::unlinked_offset), [`linked_offset`](Self::linked_offset),
    /// and [`show_when_unlinked`](Self::show_when_unlinked) properties must be non-null before
    /// the compositing phase of the pipeline.
    pub fn new(app: &mut App, link: Handle<LayerLink>) -> Handle<FollowerLayer> {
        let link = app.retain(link);
        app.create(FollowerLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            link: Some(link),
            show_when_unlinked: true,
            unlinked_offset: Offset::ZERO,
            linked_offset: Offset::ZERO,
            last_offset: None,
            last_transform: None,
            inverted_transform: Cell::new(None),
            inverse_dirty: Cell::new(true),
        })
    }

    /// The link to the [`LeaderLayer`].
    ///
    /// The same object should be provided to a [`LeaderLayer`] that is earlier in the layer
    /// tree. When this layer is composited, it will apply a transform that moves its children
    /// to match the position of the [`LeaderLayer`].
    pub fn link(self: Handle<Self>, app: &App) -> Handle<LayerLink> {
        app.get(self)
            .link
            .as_ref()
            .expect("a FollowerLayer holds its link until dispose")
            .get()
    }

    /// See [`link`](Self::link).
    pub fn set_link(self: Handle<Self>, app: &mut App, value: Handle<LayerLink>) {
        if self.link(app) == value {
            return;
        }
        let retained = app.retain(value);
        let old = app
            .get_mut(self)
            .link
            .replace(retained)
            .expect("a FollowerLayer holds its link until dispose");
        app.release(old);
    }

    /// Whether to show the layer's contents when the [`link`](Self::link) does not point to a
    /// [`LeaderLayer`].
    ///
    /// When the layer is linked, children layers are positioned such that they have the same
    /// global position as the linked [`LeaderLayer`].
    ///
    /// When the layer is not linked, then: if [`show_when_unlinked`](Self::show_when_unlinked)
    /// is true, children are positioned as if the [`FollowerLayer`] was a [`ContainerLayer`];
    /// if it is false, then children are hidden.
    ///
    /// The [`show_when_unlinked`](Self::show_when_unlinked) property must be non-null before the
    /// compositing phase of the pipeline.
    pub fn show_when_unlinked(self: Handle<Self>, app: &App) -> bool {
        app.get(self).show_when_unlinked
    }

    /// See [`show_when_unlinked`](Self::show_when_unlinked).
    pub fn set_show_when_unlinked(self: Handle<Self>, app: &mut App, value: bool) {
        app.get_mut(self).show_when_unlinked = value;
    }

    /// Offset from parent in the parent's coordinate system, used when the layer is not linked
    /// to a [`LeaderLayer`].
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    ///
    /// The [`unlinked_offset`](Self::unlinked_offset) property must be non-null before the
    /// compositing phase of the pipeline.
    ///
    /// See also:
    ///
    ///  * [`linked_offset`](Self::linked_offset), for when the layers are linked.
    pub fn unlinked_offset(self: Handle<Self>, app: &App) -> Offset {
        app.get(self).unlinked_offset
    }

    /// See [`unlinked_offset`](Self::unlinked_offset).
    pub fn set_unlinked_offset(self: Handle<Self>, app: &mut App, value: Offset) {
        app.get_mut(self).unlinked_offset = value;
    }

    /// Offset from the origin of the leader layer to the origin of the child layers, used when
    /// the layer is linked to a [`LeaderLayer`].
    ///
    /// The scene must be explicitly recomposited after this property is changed (as described at
    /// [`Layer`]).
    ///
    /// The [`linked_offset`](Self::linked_offset) property must be non-null before the
    /// compositing phase of the pipeline.
    ///
    /// See also:
    ///
    ///  * [`unlinked_offset`](Self::unlinked_offset), for when the layer is not linked.
    pub fn linked_offset(self: Handle<Self>, app: &App) -> Offset {
        app.get(self).linked_offset
    }

    /// See [`linked_offset`](Self::linked_offset).
    pub fn set_linked_offset(self: Handle<Self>, app: &mut App, value: Offset) {
        app.get_mut(self).linked_offset = value;
    }

    fn transform_offset(self: Handle<Self>, app: &App, local_position: Offset) -> Option<Offset> {
        if app.get(self).inverse_dirty.get() {
            let last = self.get_last_transform(app).expect(
                "FollowerLayer._transformOffset requires getLastTransform after addToScene",
            );
            app.get(self).inverted_transform.set(last.invert());
            app.get(self).inverse_dirty.set(false);
        }
        let inverted = app.get(self).inverted_transform.get()?;
        let storage = inverted.to_flutter_array();
        let x = local_position.dx();
        let y = local_position.dy();
        let rx = storage[0] as f64 * x + storage[4] as f64 * y + storage[12] as f64;
        let ry = storage[1] as f64 * x + storage[5] as f64 * y + storage[13] as f64;
        let linked_offset = self.linked_offset(app);
        Some(Offset::new(
            rx - linked_offset.dx(),
            ry - linked_offset.dy(),
        ))
    }

    /// The transform that was used during the last composition phase.
    ///
    /// If the [`link`](Self::link) was not linked to a [`LeaderLayer`], or if this layer has a
    /// degenerate matrix applied, then this will be `None`.
    ///
    /// This method returns a new [`Matrix4`] instance each time it is invoked.
    pub fn get_last_transform(self: Handle<Self>, app: &App) -> Option<Matrix4> {
        let last_transform = app.get(self).last_transform?;
        let last_offset = app
            .get(self)
            .last_offset
            .expect("lastOffset is set with lastTransform");
        Some(
            Matrix4::translation(-last_offset.dx() as f32, -last_offset.dy() as f32)
                .then(&last_transform),
        )
    }

    /// Call [`ContainerLayer::apply_transform`] for each layer in the provided list.
    ///
    /// The list is in reverse order (deepest first). The first layer will be treated as the
    /// child of the second, and so forth. The first layer in the list won't have
    /// `apply_transform` called on it. The first layer may be null.
    fn collect_transform_for_layer_chain(
        app: &App,
        layers: &[Option<AnyContainerLayer>],
    ) -> Matrix4 {
        let mut result = Matrix4::IDENTITY;
        for index in (1..layers.len()).rev() {
            if let Some(layer) = layers[index] {
                let child = layers[index - 1].map(AnyContainerLayer::as_layer);
                layer.apply_transform(app, child, &mut result);
            }
        }
        result
    }

    /// Find the common ancestor of two layers `a` and `b` by searching towards the root of the
    /// tree, and append each ancestor of `a` or `b` visited along the path to `ancestors_a` and
    /// `ancestors_b` respectively.
    ///
    /// Returns `None` if `a` `b` do not share a common ancestor, in which case the results in
    /// `ancestors_a` and `ancestors_b` are undefined.
    fn paths_to_common_ancestor(
        app: &App,
        a: Option<AnyLayer>,
        b: Option<AnyLayer>,
        ancestors_a: &mut Vec<Option<AnyContainerLayer>>,
        ancestors_b: &mut Vec<Option<AnyContainerLayer>>,
    ) -> Option<AnyLayer> {
        let (a, b) = (a?, b?);
        if a == b {
            return Some(a);
        }
        if a.depth(app) < b.depth(app) {
            ancestors_b.push(b.parent(app));
            return Self::paths_to_common_ancestor(
                app,
                Some(a),
                b.parent(app).map(AnyContainerLayer::as_layer),
                ancestors_a,
                ancestors_b,
            );
        } else if a.depth(app) > b.depth(app) {
            ancestors_a.push(a.parent(app));
            return Self::paths_to_common_ancestor(
                app,
                a.parent(app).map(AnyContainerLayer::as_layer),
                Some(b),
                ancestors_a,
                ancestors_b,
            );
        }
        ancestors_a.push(a.parent(app));
        ancestors_b.push(b.parent(app));
        Self::paths_to_common_ancestor(
            app,
            a.parent(app).map(AnyContainerLayer::as_layer),
            b.parent(app).map(AnyContainerLayer::as_layer),
            ancestors_a,
            ancestors_b,
        )
    }

    fn debug_check_leader_before_follower(
        app: &App,
        leader_to_common_ancestor: &[Option<AnyContainerLayer>],
        follower_to_common_ancestor: &[Option<AnyContainerLayer>],
    ) -> bool {
        if follower_to_common_ancestor.len() <= 1 {
            return false;
        }
        if leader_to_common_ancestor.len() <= 1 {
            return true;
        }
        let leader_subtree_below_ancestor = leader_to_common_ancestor
            [leader_to_common_ancestor.len() - 2]
            .expect("the leader subtree below the ancestor is a layer");
        let follower_subtree_below_ancestor = follower_to_common_ancestor
            [follower_to_common_ancestor.len() - 2]
            .expect("the follower subtree below the ancestor is a layer");
        let mut sibling = Some(leader_subtree_below_ancestor.as_layer());
        while let Some(layer) = sibling {
            if layer == follower_subtree_below_ancestor.as_layer() {
                return true;
            }
            sibling = layer.next_sibling(app);
        }
        false
    }

    /// Populate `_lastTransform` given the current state of the tree.
    fn establish_transform(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).last_transform = None;
        let Some(leader) = self.link(app).leader(app) else {
            return;
        };
        debug_assert_eq!(
            leader.as_layer().owner(app),
            self.as_layer().owner(app),
            "Linked LeaderLayer anchor is not in the same layer tree as the FollowerLayer."
        );

        let mut forward_layers = vec![Some(leader.as_container_layer())];
        let mut inverse_layers = vec![Some(self.as_container_layer())];
        let ancestor = Self::paths_to_common_ancestor(
            app,
            Some(leader.as_layer()),
            Some(self.as_layer()),
            &mut forward_layers,
            &mut inverse_layers,
        );
        debug_assert!(
            ancestor.is_some(),
            "LeaderLayer and FollowerLayer do not have a common ancestor."
        );
        debug_assert!(
            Self::debug_check_leader_before_follower(app, &forward_layers, &inverse_layers),
            "LeaderLayer anchor must come before FollowerLayer in paint order, but the reverse \
             was true."
        );

        let mut forward_transform = Self::collect_transform_for_layer_chain(app, &forward_layers);
        leader
            .as_container_layer()
            .apply_transform(app, None, &mut forward_transform);
        let linked_offset = self.linked_offset(app);
        forward_transform = forward_transform.then(&Matrix4::translation(
            linked_offset.dx() as f32,
            linked_offset.dy() as f32,
        ));

        let inverse_transform = Self::collect_transform_for_layer_chain(app, &inverse_layers);
        let Some(inverse_transform) = inverse_transform.invert() else {
            return;
        };
        app.get_mut(self).last_transform = Some(inverse_transform.then(&forward_transform));
        app.get(self).inverse_dirty.set(true);
    }
}

impl Layer for FollowerLayer {
    crate::layer_accessors!(container);

    /// A [`FollowerLayer`] copies changes from a [`LeaderLayer`] that could be anywhere in the
    /// layer tree, and that leader layer could change without notifying the follower layer.
    /// Therefore a follower layer's `add_to_scene` is always called.
    fn always_needs_add_to_scene(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let link = app
            .get_mut(self)
            .link
            .take()
            .expect("a FollowerLayer holds its link until dispose");
        ContainerLayer::dispose(self, app);
        app.release(link);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        if self.link(app).leader(app).is_none() {
            if self.show_when_unlinked(app) {
                return ContainerLayer::find_annotations(
                    self,
                    app,
                    result,
                    local_position - self.unlinked_offset(app),
                    only_first,
                );
            }
            return false;
        }
        let Some(transformed_offset) = self.transform_offset(app, local_position) else {
            return false;
        };
        ContainerLayer::find_annotations(self, app, result, transformed_offset, only_first)
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        if self.link(app).leader(app).is_none() && !self.show_when_unlinked(app) {
            app.get_mut(self).last_transform = None;
            app.get_mut(self).last_offset = None;
            app.get(self).inverse_dirty.set(true);
            self.as_layer().set_engine_layer(app, None);
            return;
        }
        self.establish_transform(app);
        if app.get(self).last_transform.is_some() {
            let unlinked_offset = self.unlinked_offset(app);
            app.get_mut(self).last_offset = Some(unlinked_offset);
            let last_transform = app
                .get(self)
                .last_transform
                .expect("lastTransform was just set");
            builder.push_transform(&last_transform);
            self.as_container_layer()
                .add_children_to_scene(app, builder);
            let engine_layer = builder.pop();
            self.as_layer().set_engine_layer(app, Some(engine_layer));
        } else {
            app.get_mut(self).last_offset = None;
            let unlinked_offset = self.unlinked_offset(app);
            let matrix =
                Matrix4::translation(unlinked_offset.dx() as f32, unlinked_offset.dy() as f32);
            builder.push_transform(&matrix);
            self.as_container_layer()
                .add_children_to_scene(app, builder);
            let engine_layer = builder.pop();
            self.as_layer().set_engine_layer(app, Some(engine_layer));
        }
        app.get(self).inverse_dirty.set(true);
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for FollowerLayer {
    crate::container_layer_accessors!();

    fn apply_transform(
        self: Handle<Self>,
        app: &App,
        child: Option<AnyLayer>,
        transform: &mut Matrix4,
    ) {
        debug_assert!(child.is_some());
        if let Some(last_transform) = app.get(self).last_transform {
            *transform = transform.then(&last_transform);
        } else {
            let unlinked_offset = self.unlinked_offset(app);
            *transform = transform.then(&Matrix4::translation(
                unlinked_offset.dx() as f32,
                unlinked_offset.dy() as f32,
            ));
        }
    }
}

/// A composited layer which annotates its children with a value. Pushing this layer to the tree
/// is the common way of adding an annotation.
///
/// An annotation is an optional object of any type that, when attached with a layer, can be
/// retrieved using [`AnyLayer::find`] or [`AnyLayer::find_all_annotations`] with a position. The
/// search process is done recursively, controlled by a concept of being opaque to a type of
/// annotation, explained in the document of [`Layer::find_annotations`].
///
/// When an annotation search arrives, this layer defers the same search to each of this layer's
/// children, respecting their opacity. Then it adds this layer's annotation if all of the
/// following restrictions are met:
///
/// * The target type must be identical to the annotated type `T`.
/// * If [`size`](Self::size) is provided, the target position must be contained within the
///   rectangle formed by [`size`](Self::size) and [`offset`](Self::offset).
///
/// This layer is opaque to a type of annotation if any child is also opaque, or if
/// [`opaque`](Self::opaque) is true and the layer's annotation is added.
pub struct AnnotatedRegionLayer {
    layer: LayerData,
    container_layer: ContainerLayerData,
    value: Rc<dyn Any>,
    size: Option<Size>,
    offset: Offset,
    opaque: bool,
}

impl AnnotatedRegionLayer {
    /// Creates a new layer that annotates its children with `value`.
    pub fn new(app: &mut App, value: Rc<dyn Any>) -> Handle<AnnotatedRegionLayer> {
        app.create(AnnotatedRegionLayer {
            layer: LayerData::new(),
            container_layer: ContainerLayerData::new(),
            value,
            size: None,
            offset: Offset::ZERO,
            opaque: false,
        })
    }

    /// The annotated object, which is added to the result if all restrictions are met.
    pub fn value(self: Handle<Self>, app: &App) -> &Rc<dyn Any> {
        &app.get(self).value
    }

    /// The size of the annotated object.
    ///
    /// If [`size`](Self::size) is provided, then the annotation is found only if the target
    /// position is contained by the rectangle formed by [`size`](Self::size) and
    /// [`offset`](Self::offset). Otherwise no such restriction is applied, and clipping can only
    /// be done by the ancestor layers.
    pub fn size(self: Handle<Self>, app: &App) -> Option<Size> {
        app.get(self).size
    }

    /// See [`size`](Self::size).
    pub fn set_size(self: Handle<Self>, app: &mut App, value: Option<Size>) {
        app.get_mut(self).size = value;
    }

    /// The position of the annotated object.
    ///
    /// The [`offset`](Self::offset) defaults to [`Offset::ZERO`] if not provided, and is ignored
    /// if [`size`](Self::size) is not set.
    ///
    /// The [`offset`](Self::offset) only offsets the clipping rectangle, and does not affect how
    /// the painting or annotation search is propagated to its children.
    pub fn offset(self: Handle<Self>, app: &App) -> Offset {
        app.get(self).offset
    }

    /// See [`offset`](Self::offset).
    pub fn set_offset(self: Handle<Self>, app: &mut App, value: Offset) {
        app.get_mut(self).offset = value;
    }

    /// Whether the annotation of this layer should be opaque during an annotation search of type
    /// `T`, preventing siblings visually behind it from being searched.
    ///
    /// If [`opaque`](Self::opaque) is true, and this layer does add its annotation
    /// [`value`](Self::value), then the layer will always be opaque during the search.
    ///
    /// If [`opaque`](Self::opaque) is false, or if this layer does not add its annotation, then
    /// the opacity of this layer will be the one returned by the children, meaning that it will
    /// be opaque if any child is opaque.
    ///
    /// The [`opaque`](Self::opaque) defaults to false.
    ///
    /// The [`opaque`](Self::opaque) is effectively useless during [`AnyLayer::find`] (more
    /// specifically, [`Layer::find_annotations`] with `only_first: true`), since the search
    /// process then skips the remaining tree after finding the first annotation.
    ///
    /// See also:
    ///
    ///  * [`Layer::find_annotations`], which explains the concept of being opaque to a type of
    ///    annotation as the return value.
    ///  * `HitTestBehavior`, which controls similar logic when hit-testing in the render tree.
    pub fn opaque(self: Handle<Self>, app: &App) -> bool {
        app.get(self).opaque
    }

    /// See [`opaque`](Self::opaque).
    pub fn set_opaque(self: Handle<Self>, app: &mut App, value: bool) {
        app.get_mut(self).opaque = value;
    }
}

impl Layer for AnnotatedRegionLayer {
    crate::layer_accessors!(container);

    fn supports_rasterization(self: Handle<Self>, app: &App) -> bool {
        ContainerLayer::supports_rasterization(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ContainerLayer::dispose(self, app);
    }

    fn attach(self: Handle<Self>, app: &mut App, owner: HandleId) {
        ContainerLayer::attach(self, app, owner);
    }

    fn detach(self: Handle<Self>, app: &mut App) {
        ContainerLayer::detach(self, app);
    }

    fn redepth_children(self: Handle<Self>, app: &mut App) {
        ContainerLayer::redepth_children(self, app);
    }

    /// Searches the subtree for annotations of the searched type at the location `local_position`,
    /// then adds the annotation [`value`](Self::value) if applicable.
    ///
    /// This method always searches its children, and if any child returns `true`, the remaining
    /// children are skipped. Regardless of what the children return, this method then adds this
    /// layer's annotation if all of the following restrictions are met:
    ///
    /// * The target type must be identical to the annotated type `T`.
    /// * If [`size`](Self::size) is provided, the target position must be contained within the
    ///   rectangle formed by [`size`](Self::size) and [`offset`](Self::offset).
    ///
    /// This search process respects `only_first`, meaning that when `only_first` is true, the
    /// search will stop when it finds the first annotation from the children, and the layer's
    /// own annotation is checked only when none is given by the children.
    ///
    /// The return value is true if any child returns `true`, or if [`opaque`](Self::opaque) is
    /// true and the layer's annotation is added.
    ///
    /// For explanation of layer annotations, parameters and return value, refer to
    /// [`Layer::find_annotations`].
    fn find_annotations(
        self: Handle<Self>,
        app: &App,
        result: &mut AnnotationSearch,
        local_position: Offset,
        only_first: bool,
    ) -> bool {
        let mut is_absorbed =
            ContainerLayer::find_annotations(self, app, result, local_position, only_first);
        if !result.is_empty() && only_first {
            return is_absorbed;
        }
        let (value, size, offset, opaque) = {
            let this = app.get(self);
            (Rc::clone(&this.value), this.size, this.offset, this.opaque)
        };
        if let Some(size) = size
            && !(offset & size).contains(local_position)
        {
            return is_absorbed;
        }
        if result.accepts(&*value) {
            is_absorbed = is_absorbed || opaque;
            result.add(value, local_position - offset);
        }
        is_absorbed
    }

    fn add_to_scene(self: Handle<Self>, app: &mut App, builder: &mut SceneBuilder) {
        ContainerLayer::add_to_scene(self, app, builder);
    }

    fn fire_composition_callbacks(self: Handle<Self>, app: &mut App, include_children: bool) {
        ContainerLayer::fire_composition_callbacks(self, app, include_children);
    }
}

impl ContainerLayer for AnnotatedRegionLayer {
    crate::container_layer_accessors!();
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;

    use super::*;

    fn offset_layer(app: &mut App) -> AnyLayer {
        OffsetLayer::new(app, Offset::ZERO).as_layer()
    }

    #[test]
    fn append_links_children_in_order_and_adopts_them() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let parent = OffsetLayer::new(&mut app, Offset::ZERO).as_container_layer();
        let a = offset_layer(&mut app);
        let b = offset_layer(&mut app);
        parent.append(&mut app, a);
        parent.append(&mut app, b);
        assert_eq!(parent.first_child(&app), Some(a));
        assert_eq!(parent.last_child(&app), Some(b));
        assert_eq!(a.next_sibling(&app), Some(b));
        assert_eq!(b.previous_sibling(&app), Some(a));
        assert_eq!(a.parent(&app), Some(parent));
        assert_eq!(a.depth(&app), 1);
        assert_eq!(a.debug_handle_count(&app), 1, "the parent handle");
    }

    #[test]
    fn a_layer_is_disposed_when_its_last_handle_is_released() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let layer = offset_layer(&mut app);
        let handle = app.create(LayerHandle::<AnyLayer>::new());
        LayerHandle::set_layer(&mut app, |app| app.get_mut(handle), Some(layer));
        assert_eq!(layer.debug_handle_count(&app), 1);
        LayerHandle::set_layer(&mut app, |app| app.get_mut(handle), None);
        assert!(layer.debug_disposed(&app));
        assert!(
            !app.contains(layer.id()),
            "dispose destroys the arena entry"
        );
    }

    #[test]
    fn removing_a_child_releases_the_parent_handle_and_disposes_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let parent = OffsetLayer::new(&mut app, Offset::ZERO).as_container_layer();
        let child = offset_layer(&mut app);
        parent.append(&mut app, child);
        child.remove(&mut app);
        assert!(!parent.has_children(&app));
        assert!(child.debug_disposed(&app));
    }

    #[test]
    fn a_child_kept_by_another_handle_survives_removal() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let parent = OffsetLayer::new(&mut app, Offset::ZERO).as_container_layer();
        let child = offset_layer(&mut app);
        let handle = app.create(LayerHandle::<AnyLayer>::new());
        LayerHandle::set_layer(&mut app, |app| app.get_mut(handle), Some(child));
        parent.append(&mut app, child);
        child.remove(&mut app);
        assert!(!child.debug_disposed(&app));
        assert_eq!(child.parent(&app), None);
        LayerHandle::set_layer(&mut app, |app| app.get_mut(handle), None);
        assert!(child.debug_disposed(&app));
    }

    #[test]
    fn attach_and_detach_walk_the_subtree() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = app.create(());
        let root = OffsetLayer::new(&mut app, Offset::ZERO).as_container_layer();
        let child = offset_layer(&mut app);
        root.append(&mut app, child);
        root.as_layer().attach(&mut app, owner.id());
        assert!(child.attached(&app));
        let late = offset_layer(&mut app);
        root.append(&mut app, late);
        assert!(
            late.attached(&app),
            "appending to an attached tree attaches"
        );
        root.as_layer().detach(&mut app);
        assert!(!child.attached(&app));
        assert!(!late.attached(&app));
    }

    #[test]
    fn disposing_a_container_disposes_children_it_alone_held() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let parent = OffsetLayer::new(&mut app, Offset::ZERO).as_container_layer();
        let child = offset_layer(&mut app);
        parent.append(&mut app, child);
        let handle = app.create(LayerHandle::<AnyContainerLayer>::new());
        LayerHandle::set_layer(&mut app, |app| app.get_mut(handle), Some(parent));
        LayerHandle::set_layer(&mut app, |app| app.get_mut(handle), None);
        assert!(parent.as_layer().debug_disposed(&app));
        assert!(child.debug_disposed(&app));
    }

    #[test]
    fn a_picture_layer_is_a_leaf_that_holds_its_picture() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let layer = PictureLayer::new(&mut app, Rect::from_ltwh(0.0, 0.0, 10.0, 10.0));
        let picture = Arc::new(inset_embedder::Canvas::new().build());
        layer.set_picture(&mut app, Some(picture));
        assert!(layer.picture(&app).is_some());
        assert!(layer.as_layer().as_container_layer().is_none());
        let mut builder = SceneBuilder::new();
        layer.as_layer().add_to_scene(&mut app, &mut builder);
        let _scene = builder.build();
    }

    #[test]
    fn composition_callbacks_fire_on_build_scene_and_detach() {
        use std::cell::Cell;
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = app.create(());
        let root = OffsetLayer::new(&mut app, Offset::ZERO).as_container_layer();
        let child = offset_layer(&mut app);
        root.append(&mut app, child);
        let fired = Rc::new(Cell::new(0));
        let remove = {
            let fired = Rc::clone(&fired);
            child
                .add_composition_callback(&mut app, Rc::new(move |_, _| fired.set(fired.get() + 1)))
        };
        assert!(root.as_layer().subtree_has_composition_callbacks(&app));
        let _scene = root.build_scene(&mut app, SceneBuilder::new());
        assert_eq!(fired.get(), 1);
        root.as_layer().attach(&mut app, owner.id());
        root.as_layer().detach(&mut app);
        assert_eq!(fired.get(), 2, "detach fires once more");
        remove(&mut app);
        assert!(!root.as_layer().subtree_has_composition_callbacks(&app));
    }

    #[test]
    fn find_walks_children_back_to_front_through_offsets() {
        struct Tag(&'static str);
        struct Annotated {
            layer: LayerData,
            tag: Rc<Tag>,
        }
        impl Layer for Annotated {
            crate::layer_accessors!();
            fn find_annotations(
                self: Handle<Self>,
                app: &App,
                result: &mut AnnotationSearch,
                local_position: Offset,
                _only_first: bool,
            ) -> bool {
                let tag = Rc::clone(&app.get(self).tag);
                if result.accepts(&*tag) {
                    result.add(tag, local_position);
                }
                false
            }
            fn add_to_scene(self: Handle<Self>, _app: &mut App, _builder: &mut SceneBuilder) {}
        }
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let root = OffsetLayer::new(&mut app, Offset::new(10.0, 0.0)).as_container_layer();
        for name in ["back", "front"] {
            let layer = app.create(Annotated {
                layer: LayerData::new(),
                tag: Rc::new(Tag(name)),
            });
            root.append(&mut app, layer.as_layer());
        }
        let found = root.as_layer().find::<Tag>(&app, Offset::new(15.0, 5.0));
        assert_eq!(found.map(|tag| tag.0), Some("front"));
        let all = root
            .as_layer()
            .find_all_annotations::<Tag>(&app, Offset::new(15.0, 5.0));
        assert_eq!(all.entries().len(), 2);
        assert_eq!(all.entries()[0].local_position, Offset::new(5.0, 5.0));
        assert!(root.as_layer().find::<String>(&app, Offset::ZERO).is_none());
    }

    /// An opacity layer over one leaf under a root, with a frame's scene built: the shape
    /// retained rendering acts on.
    fn retained_tree(app: &mut App) -> (Handle<OffsetLayer>, Handle<OpacityLayer>) {
        let root = OffsetLayer::new(app, Offset::ZERO);
        let opacity = OpacityLayer::new(app);
        opacity.set_alpha(app, 128);
        let leaf = offset_layer(app);
        opacity.as_container_layer().append(app, leaf);
        root.as_container_layer().append(app, opacity.as_layer());
        let _scene = root
            .as_container_layer()
            .build_scene(app, SceneBuilder::new());
        (root, opacity)
    }

    #[test]
    fn an_unchanged_subtree_is_retained_across_scenes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (root, opacity) = retained_tree(&mut app);
        let recorded = opacity.as_layer().engine_layer(&app).expect("recorded");
        assert!(!opacity.as_layer().needs_add_to_scene(&app));

        let scene = root
            .as_container_layer()
            .build_scene(&mut app, SceneBuilder::new());
        let retained = opacity.as_layer().engine_layer(&app).expect("recorded");
        assert!(
            Arc::ptr_eq(&recorded, &retained),
            "the same list is retained"
        );
        let root_list = root
            .as_layer()
            .engine_layer(&app)
            .expect("the root re-records");
        assert!(
            root_list.ops().iter().any(|op| matches!(
                op,
                inset_embedder::valo::Op::DrawDisplayList { list, .. } if Arc::ptr_eq(list, &retained)
            )),
            "embedded again: {:?}",
            root_list.ops()
        );
        assert_eq!(
            inset_embedder::flattened_ops(&scene)
                .iter()
                .filter(|op| matches!(op, inset_embedder::valo::Op::SaveLayer { .. }))
                .count(),
            1,
            "the retained opacity layer still draws"
        );
    }

    #[test]
    fn a_changed_property_re_records_the_subtree() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (root, opacity) = retained_tree(&mut app);
        let recorded = opacity.as_layer().engine_layer(&app).expect("recorded");

        opacity.set_alpha(&mut app, 200);
        assert!(opacity.as_layer().needs_add_to_scene(&app));
        let _scene = root
            .as_container_layer()
            .build_scene(&mut app, SceneBuilder::new());
        let again = opacity.as_layer().engine_layer(&app).expect("recorded");
        assert!(
            !Arc::ptr_eq(&recorded, &again),
            "a new list for the new alpha"
        );
        assert!(!opacity.as_layer().needs_add_to_scene(&app));
    }

    #[test]
    fn appending_a_child_marks_the_parent_and_its_ancestors_at_build() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (root, opacity) = retained_tree(&mut app);
        assert!(!root.as_layer().needs_add_to_scene(&app));

        let leaf = offset_layer(&mut app);
        opacity.as_container_layer().append(&mut app, leaf);
        assert!(opacity.as_layer().needs_add_to_scene(&app));
        assert!(
            !root.as_layer().needs_add_to_scene(&app),
            "the mark does not climb until the scene is built"
        );
        root.as_layer().update_subtree_needs_add_to_scene(&mut app);
        assert!(root.as_layer().needs_add_to_scene(&app));
    }

    #[test]
    fn a_follower_layer_always_re_records() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let link = LayerLink::new(&mut app);
        let follower = FollowerLayer::new(&mut app, link);
        let child = offset_layer(&mut app);
        follower.as_container_layer().append(&mut app, child);
        for _ in 0..2 {
            let _scene = follower
                .as_container_layer()
                .build_scene(&mut app, SceneBuilder::new());
            assert!(
                follower.as_layer().needs_add_to_scene(&app)
                    || follower.as_layer().always_needs_add_to_scene(&app)
            );
        }
        let first = follower.as_layer().engine_layer(&app).expect("recorded");
        let _scene = follower
            .as_container_layer()
            .build_scene(&mut app, SceneBuilder::new());
        let second = follower.as_layer().engine_layer(&app).expect("recorded");
        assert!(!Arc::ptr_eq(&first, &second), "never retained");
    }

    fn tag(app: &mut App, n: u32) -> Handle<AnnotatedRegionLayer> {
        AnnotatedRegionLayer::new(app, Rc::new(n))
    }

    #[test]
    fn clip_layers_reject_annotations_outside_the_clip() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let rect = ClipRectLayer::new(&mut app);
        rect.set_clip_rect(&mut app, Some(Rect::from_ltwh(0.0, 0.0, 50.0, 50.0)));
        let child = tag(&mut app, 7);
        rect.as_container_layer().append(&mut app, child.as_layer());
        assert_eq!(
            rect.as_layer()
                .find::<u32>(&app, Offset::new(10.0, 10.0))
                .as_deref(),
            Some(&7)
        );
        assert!(
            rect.as_layer()
                .find::<u32>(&app, Offset::new(60.0, 10.0))
                .is_none(),
            "outside the clip"
        );

        let rrect = ClipRRectLayer::new(&mut app);
        rrect.set_clip_rrect(
            &mut app,
            Some(RRect::from_rect_and_radius(
                Rect::from_ltwh(0.0, 0.0, 50.0, 50.0),
                inset_embedder::Radius::circular(4.0),
            )),
        );
        let child = tag(&mut app, 8);
        rrect
            .as_container_layer()
            .append(&mut app, child.as_layer());
        assert_eq!(
            rrect
                .as_layer()
                .find::<u32>(&app, Offset::new(10.0, 10.0))
                .as_deref(),
            Some(&8)
        );
        assert!(
            rrect
                .as_layer()
                .find::<u32>(&app, Offset::new(60.0, 10.0))
                .is_none()
        );

        let mut path = inset_embedder::PathBuilder::new();
        path.rect(Rect::from_ltwh(0.0, 0.0, 50.0, 50.0).into());
        let clip_path = ClipPathLayer::new(&mut app);
        clip_path.set_clip_path(&mut app, Some(path.build()));
        let child = tag(&mut app, 9);
        clip_path
            .as_container_layer()
            .append(&mut app, child.as_layer());
        assert_eq!(
            clip_path
                .as_layer()
                .find::<u32>(&app, Offset::new(10.0, 10.0))
                .as_deref(),
            Some(&9)
        );
        assert!(
            clip_path
                .as_layer()
                .find::<u32>(&app, Offset::new(60.0, 10.0))
                .is_none()
        );
    }

    #[test]
    fn transform_layer_transforms_the_annotation_position() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let layer = TransformLayer::new(&mut app);
        layer.set_transform(&mut app, Matrix4::translation(10.0, 0.0));
        let region = tag(&mut app, 7);
        region.set_size(&mut app, Some(Size::new(20.0, 20.0)));
        layer
            .as_container_layer()
            .append(&mut app, region.as_layer());
        assert!(
            layer
                .as_layer()
                .find::<u32>(&app, Offset::new(15.0, 5.0))
                .is_some()
        );
        assert!(
            layer
                .as_layer()
                .find::<u32>(&app, Offset::new(5.0, 5.0))
                .is_none(),
            "left of the translated region"
        );
    }

    #[test]
    fn opacity_layer_build_scene_succeeds_at_255_and_128() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        for alpha in [255, 128] {
            let layer = OpacityLayer::new(&mut app);
            layer.set_alpha(&mut app, alpha);
            let child = OffsetLayer::new(&mut app, Offset::ZERO);
            layer
                .as_container_layer()
                .append(&mut app, child.as_layer());
            let _scene = layer
                .as_container_layer()
                .build_scene(&mut app, SceneBuilder::new());
        }
        let empty = OpacityLayer::new(&mut app);
        empty.set_alpha(&mut app, 128);
        let _scene = empty
            .as_container_layer()
            .build_scene(&mut app, SceneBuilder::new());
    }

    #[test]
    fn layer_link_register_unregister_and_leader_getter() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = app.create(());
        let link = LayerLink::new(&mut app);
        assert!(link.leader(&app).is_none());
        let leader = LeaderLayer::new(&mut app, link);
        leader.as_layer().attach(&mut app, owner.id());
        assert_eq!(link.leader(&app), Some(leader));
        leader.as_layer().detach(&mut app);
        assert!(link.leader(&app).is_none());
    }

    #[test]
    fn leader_layer_attach_registers_and_detach_unregisters() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = app.create(());
        let root = OffsetLayer::new(&mut app, Offset::ZERO);
        let link = LayerLink::new(&mut app);
        let leader = LeaderLayer::new(&mut app, link);
        root.as_container_layer()
            .append(&mut app, leader.as_layer());
        root.as_layer().attach(&mut app, owner.id());
        assert_eq!(link.leader(&app), Some(leader));
        root.as_layer().detach(&mut app);
        assert!(link.leader(&app).is_none());
    }

    #[test]
    fn a_replaced_leader_leaves_the_links_new_leader_in_place() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = app.create(());
        let link = LayerLink::new(&mut app);
        let leader1 = LeaderLayer::new(&mut app, link);
        let leader2 = LeaderLayer::new(&mut app, link);
        leader1.as_layer().attach(&mut app, owner.id());
        leader2.as_layer().attach(&mut app, owner.id());
        assert_eq!(link.leader(&app), Some(leader2));
        leader2.as_layer().detach(&mut app);
        leader1.as_layer().detach(&mut app);
        assert!(link.leader(&app).is_none());
    }

    #[test]
    fn switching_the_link_of_an_attached_leader_does_not_crash() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = app.create(());
        let link = LayerLink::new(&mut app);
        let leader = LeaderLayer::new(&mut app, link);
        leader.as_layer().attach(&mut app, owner.id());
        let link2 = LayerLink::new(&mut app);
        leader.set_link(&mut app, link2);
        leader.as_layer().detach(&mut app);
        assert_eq!(leader.link(&app), link2);
        assert!(link.leader(&app).is_none());
        assert!(link2.leader(&app).is_none());
    }

    #[test]
    fn follower_get_last_transform_after_build_scene_matches_the_leader_offset() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = app.create(());
        let root = OffsetLayer::new(&mut app, Offset::ZERO);
        let link = LayerLink::new(&mut app);
        let leader = LeaderLayer::new(&mut app, link);
        leader.set_offset(&mut app, Offset::new(10.0, 20.0));
        let follower = FollowerLayer::new(&mut app, link);
        root.as_container_layer()
            .append(&mut app, leader.as_layer());
        root.as_container_layer()
            .append(&mut app, follower.as_layer());
        root.as_layer().attach(&mut app, owner.id());
        let _scene = root
            .as_container_layer()
            .build_scene(&mut app, SceneBuilder::new());
        assert_eq!(
            follower.get_last_transform(&app).unwrap(),
            Matrix4::translation(10.0, 20.0)
        );
    }

    #[test]
    fn follower_with_show_when_unlinked_false_pushes_nothing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let link = LayerLink::new(&mut app);
        let follower = FollowerLayer::new(&mut app, link);
        follower.set_show_when_unlinked(&mut app, false);
        let child = offset_layer(&mut app);
        follower.as_container_layer().append(&mut app, child);
        let _scene = follower
            .as_container_layer()
            .build_scene(&mut app, SceneBuilder::new());
        assert!(follower.get_last_transform(&app).is_none());
    }

    #[test]
    fn annotated_region_layer_find_with_size_offset_and_opaque() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let root = OffsetLayer::new(&mut app, Offset::ZERO);
        let behind = tag(&mut app, 1);
        let front = tag(&mut app, 2);
        front.set_opaque(&mut app, true);
        root.as_container_layer()
            .append(&mut app, behind.as_layer());
        root.as_container_layer().append(&mut app, front.as_layer());
        let result = root
            .as_layer()
            .find_all_annotations::<u32>(&app, Offset::ZERO);
        assert_eq!(
            result
                .annotations()
                .map(|value| **value)
                .collect::<Vec<_>>(),
            vec![2],
            "an opaque region hides its siblings"
        );

        let root = OffsetLayer::new(&mut app, Offset::ZERO);
        let first = tag(&mut app, 1);
        let second = tag(&mut app, 2);
        root.as_container_layer().append(&mut app, first.as_layer());
        root.as_container_layer()
            .append(&mut app, second.as_layer());
        let result = root
            .as_layer()
            .find_all_annotations::<u32>(&app, Offset::ZERO);
        assert_eq!(
            result
                .annotations()
                .map(|value| **value)
                .collect::<Vec<_>>(),
            vec![2, 1],
            "front to back when neither is opaque"
        );

        let sized = tag(&mut app, 3);
        sized.set_size(&mut app, Some(Size::new(20.0, 20.0)));
        sized.set_offset(&mut app, Offset::new(90.0, 90.0));
        assert!(
            sized
                .as_layer()
                .find::<u32>(&app, Offset::new(95.0, 95.0))
                .is_some()
        );
        assert!(
            sized.as_layer().find::<u32>(&app, Offset::ZERO).is_none(),
            "outside the sized region"
        );
    }
}

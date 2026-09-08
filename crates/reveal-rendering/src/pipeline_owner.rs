//! Flutter counterpart: `rendering/object.dart` (`PipelineOwner`).
//!
//! Semantics callbacks and the semantics half of [`PipelineManifold`] wait.

use std::rc::Rc;

use reveal_foundation::{App, Handle, Listener};

use crate::layer::ErasedLayer;
use crate::object::AnyRenderObject;
use crate::painting_context::PaintingContext;

/// Manages a tree of [`PipelineOwner`]s.
///
/// All [`PipelineOwner`]s within a tree are attached to the same
/// [`PipelineManifold`], which gives them access to shared functionality such
/// as requesting a visual update (by calling
/// [`request_visual_update`](Self::request_visual_update)). As such, the
/// [`PipelineManifold`] gives the [`PipelineOwner`]s access to functionality
/// usually provided by the bindings without tying the [`PipelineOwner`]s to a
/// particular binding implementation.
///
/// The root of the [`PipelineOwner`] tree is attached to a [`PipelineManifold`] by
/// passing the manifold to [`PipelineOwner::attach`]. Children are attached to the
/// same [`PipelineManifold`] as their parent when they are adopted via
/// [`PipelineOwner::adopt_child`].
///
/// Dart's `semanticsEnabled` and the listeners that report its changes wait with
/// semantics.
pub trait PipelineManifold {
    /// Called by a [`PipelineOwner`] connected to this [`PipelineManifold`] when a
    /// `RenderObject` associated with that pipeline owner wishes to update its
    /// visual appearance.
    ///
    /// Typical implementations of this function will schedule a task to flush the
    /// various stages of the pipeline. This function might be called multiple
    /// times in quick succession. Implementations should take care to discard
    /// duplicate calls quickly.
    ///
    /// A [`PipelineOwner`] connected to this [`PipelineManifold`] will call its
    /// `on_need_visual_update` callback instead of this method if it has been
    /// configured with one ([`PipelineOwner::new`]).
    ///
    /// See also:
    ///
    ///  * `SchedulerBinding::ensure_visual_update`, which [`PipelineManifold`]
    ///    implementations typically call to implement this method.
    fn request_visual_update(&self, app: &mut App);
}

/// The pipeline owner manages the rendering pipeline.
///
/// Flutter's counterpart is `PipelineOwner`.
pub struct PipelineOwner {
    on_need_visual_update: Option<Listener>,
    manifold: Option<Rc<dyn PipelineManifold>>,
    root_node: Option<AnyRenderObject>,
    nodes_needing_layout: Vec<AnyRenderObject>,
    nodes_needing_compositing_bits_update: Vec<AnyRenderObject>,
    should_merge_dirty_nodes: bool,
    /// Dart's `_debugAllowMutationsToDirtySubtrees`.
    debug_allow_mutations_to_dirty_subtrees: bool,
    debug_doing_layout: bool,
    debug_doing_child_layout: bool,
    nodes_needing_paint: Vec<AnyRenderObject>,
    debug_doing_paint: bool,
    children: Vec<Handle<PipelineOwner>>,
    debug_parent: Option<Handle<PipelineOwner>>,
}

impl PipelineOwner {
    /// Creates a pipeline owner.
    ///
    /// Typically created by the binding, but can be created separately to drive
    /// off-screen render objects through the rendering pipeline.
    pub fn new(app: &mut App, on_need_visual_update: Option<Listener>) -> Handle<PipelineOwner> {
        app.create(PipelineOwner {
            on_need_visual_update,
            manifold: None,
            root_node: None,
            nodes_needing_layout: Vec::new(),
            nodes_needing_compositing_bits_update: Vec::new(),
            should_merge_dirty_nodes: false,
            debug_allow_mutations_to_dirty_subtrees: false,
            debug_doing_layout: false,
            debug_doing_child_layout: false,
            nodes_needing_paint: Vec::new(),
            debug_doing_paint: false,
            children: Vec::new(),
            debug_parent: None,
        })
    }

    /// Calls the `on_need_visual_update` callback given to [`new`](Self::new) if there is
    /// one, otherwise asks the [`PipelineManifold`] this owner is attached to.
    ///
    /// Used to notify the pipeline owner that an associated render object wishes
    /// to update its visual appearance.
    pub fn request_visual_update(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = app.get(self).on_need_visual_update.clone() {
            callback.call(app);
        } else if let Some(manifold) = app.get(self).manifold.clone() {
            manifold.request_visual_update(app);
        }
    }

    /// The unique object managed by this pipeline that has no parent.
    pub fn root_node(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        app.get(self).root_node
    }

    /// Sets the unique object managed by this pipeline that has no parent.
    pub fn set_root_node(self: Handle<Self>, app: &mut App, value: Option<AnyRenderObject>) {
        let current = app.get(self).root_node;
        if current == value {
            return;
        }
        if let Some(old) = current {
            old.detach(app);
        }
        app.get_mut(self).root_node = value;
        if let Some(new) = value {
            new.attach(app, self);
        }
    }

    /// Relayout boundaries which need to be laid out in the next
    /// [`flush_layout`](Self::flush_layout) pass.
    pub fn nodes_needing_layout(self: Handle<Self>, app: &App) -> Vec<AnyRenderObject> {
        app.get(self).nodes_needing_layout.clone()
    }

    pub(crate) fn add_node_needing_layout(
        self: Handle<Self>,
        app: &mut App,
        node: AnyRenderObject,
    ) {
        app.get_mut(self).nodes_needing_layout.push(node);
    }

    pub(crate) fn add_node_needing_compositing_bits_update(
        self: Handle<Self>,
        app: &mut App,
        node: AnyRenderObject,
    ) {
        app.get_mut(self)
            .nodes_needing_compositing_bits_update
            .push(node);
    }

    pub(crate) fn remove_node_needing_paint(
        self: Handle<Self>,
        app: &mut App,
        node: AnyRenderObject,
    ) {
        app.get_mut(self)
            .nodes_needing_paint
            .retain(|candidate| *candidate != node);
    }

    /// Whether this pipeline is currently in the layout phase.
    ///
    /// Always `false` when debug assertions are disabled.
    pub fn debug_doing_layout(self: Handle<Self>, app: &App) -> bool {
        if !cfg!(debug_assertions) {
            return false;
        }
        app.get(self).debug_doing_layout
    }

    /// See [`AnyRenderObject::invoke_layout_callback`]: runs `callback` with mutations to
    /// dirty subtrees allowed, then asks [`flush_layout`](Self::flush_layout) to merge the
    /// nodes it dirtied before continuing.
    pub(crate) fn enable_mutations_to_dirty_subtrees(
        self: Handle<Self>,
        app: &mut App,
        callback: impl FnOnce(&mut App),
    ) {
        debug_assert!(app.get(self).debug_doing_layout);
        let old_state = app.get(self).debug_allow_mutations_to_dirty_subtrees;
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_allow_mutations_to_dirty_subtrees = true;
        }
        callback(app);
        app.get_mut(self).should_merge_dirty_nodes = true;
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_allow_mutations_to_dirty_subtrees = old_state;
        }
    }

    /// Update the layout information for all dirty render objects.
    pub fn flush_layout(self: Handle<Self>, app: &mut App) {
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_doing_layout = true;
        }
        while !app.get(self).nodes_needing_layout.is_empty() {
            debug_assert!(!app.get(self).should_merge_dirty_nodes);
            let mut dirty_nodes = std::mem::take(&mut app.get_mut(self).nodes_needing_layout);
            dirty_nodes.sort_by_key(|node| node.depth(app));
            let mut i = 0;
            while i < dirty_nodes.len() {
                if app.get(self).should_merge_dirty_nodes {
                    app.get_mut(self).should_merge_dirty_nodes = false;
                    if !app.get(self).nodes_needing_layout.is_empty() {
                        app.get_mut(self)
                            .nodes_needing_layout
                            .extend(dirty_nodes.drain(i..));
                        break;
                    }
                }
                let node = dirty_nodes[i];
                if node.needs_layout(app) && node.owner(app) == Some(self) {
                    node.layout_without_resize(app);
                }
                i += 1;
            }
            app.get_mut(self).should_merge_dirty_nodes = false;
        }
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_doing_child_layout = true;
        }
        let children = app.get(self).children.clone();
        for child in children {
            child.flush_layout(app);
        }
        debug_assert!(
            app.get(self).nodes_needing_layout.is_empty(),
            "Child PipelineOwners must not dirty nodes in their parent."
        );
        app.get_mut(self).should_merge_dirty_nodes = false;
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_doing_layout = false;
            app.get_mut(self).debug_doing_child_layout = false;
        }
    }

    /// Updates the [`AnyRenderObject::needs_compositing`] bits.
    ///
    /// Called as part of the rendering pipeline after [`flush_layout`](Self::flush_layout) and
    /// before [`flush_paint`](Self::flush_paint).
    pub fn flush_compositing_bits(self: Handle<Self>, app: &mut App) {
        let mut dirty_nodes =
            std::mem::take(&mut app.get_mut(self).nodes_needing_compositing_bits_update);
        dirty_nodes.sort_by_key(|node| node.depth(app));
        for node in dirty_nodes {
            if node.needs_compositing_bits_update(app) && node.owner(app) == Some(self) {
                node.update_compositing_bits(app);
            }
        }
        let children = app.get(self).children.clone();
        for child in children {
            child.flush_compositing_bits(app);
        }
        debug_assert!(
            app.get(self)
                .nodes_needing_compositing_bits_update
                .is_empty(),
            "Child PipelineOwners must not dirty nodes in their parent."
        );
    }

    /// Nodes with a dirty layer or paint state, to be updated in the next
    /// [`flush_paint`](Self::flush_paint) pass.
    pub fn nodes_needing_paint(self: Handle<Self>, app: &App) -> Vec<AnyRenderObject> {
        app.get(self).nodes_needing_paint.clone()
    }

    pub(crate) fn add_node_needing_paint(self: Handle<Self>, app: &mut App, node: AnyRenderObject) {
        app.get_mut(self).nodes_needing_paint.push(node);
    }

    /// Whether this pipeline is currently in the paint phase.
    ///
    /// Only meaningful when asserts are enabled.
    pub fn debug_doing_paint(self: Handle<Self>, app: &App) -> bool {
        app.get(self).debug_doing_paint
    }

    /// Update the display lists for all render objects.
    ///
    /// This function is one of the core stages of the rendering pipeline. Painting occurs after
    /// layout and before the scene is recomposited so that scene is composited with up-to-date
    /// display lists for every render object.
    pub fn flush_paint(self: Handle<Self>, app: &mut App) {
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_doing_paint = true;
        }
        let mut dirty_nodes = std::mem::take(&mut app.get_mut(self).nodes_needing_paint);
        dirty_nodes.sort_by_key(|node| std::cmp::Reverse(node.depth(app)));
        for node in dirty_nodes {
            debug_assert!(node.layer(app).is_some() || !cfg!(debug_assertions));
            if (node.needs_paint(app) || node.needs_composited_layer_update(app))
                && node.owner(app) == Some(self)
            {
                if node
                    .layer(app)
                    .is_some_and(|layer| layer.as_layer().attached(app))
                {
                    debug_assert!(node.is_repaint_boundary(app));
                    if node.needs_paint(app) {
                        PaintingContext::repaint_composited_child(app, node);
                    } else {
                        PaintingContext::update_layer_properties(app, node);
                    }
                } else {
                    node.skipped_painting_on_layer(app);
                }
            }
        }
        let children = app.get(self).children.clone();
        for child in children {
            child.flush_paint(app);
        }
        debug_assert!(
            app.get(self).nodes_needing_paint.is_empty(),
            "Child PipelineOwners must not dirty nodes in their parent."
        );
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_doing_paint = false;
        }
    }

    /// Mark this [`PipelineOwner`] as attached to the given [`PipelineManifold`].
    ///
    /// Typically, this is only called directly on the root [`PipelineOwner`].
    /// Children are automatically attached to their parent's [`PipelineManifold`]
    /// when [`adopt_child`](Self::adopt_child) is called.
    pub fn attach(self: Handle<Self>, app: &mut App, manifold: Rc<dyn PipelineManifold>) {
        debug_assert!(app.get(self).manifold.is_none());
        app.get_mut(self).manifold = Some(Rc::clone(&manifold));
        let children = app.get(self).children.clone();
        for child in children {
            child.attach(app, Rc::clone(&manifold));
        }
    }

    /// Mark this [`PipelineOwner`] as detached.
    ///
    /// Typically, this is only called directly on the root [`PipelineOwner`].
    /// Children are automatically detached from their parent's [`PipelineManifold`]
    /// when [`drop_child`](Self::drop_child) is called.
    pub fn detach(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).manifold.is_some());
        app.get_mut(self).manifold = None;
        let children = app.get(self).children.clone();
        for child in children {
            child.detach(app);
        }
    }

    /// Adds `child` to this [`PipelineOwner`].
    pub fn adopt_child(self: Handle<Self>, app: &mut App, child: Handle<PipelineOwner>) {
        debug_assert!(app.get(child).debug_parent.is_none());
        debug_assert!(!app.get(self).children.contains(&child));
        debug_assert!(
            !app.get(self).debug_doing_child_layout,
            "Cannot modify child list after layout."
        );
        app.get_mut(self).children.push(child);
        if cfg!(debug_assertions) {
            app.get_mut(child).debug_parent = Some(self);
        }
        if let Some(manifold) = app.get(self).manifold.clone() {
            child.attach(app, manifold);
        }
    }

    /// Removes a child [`PipelineOwner`] previously added via
    /// [`adopt_child`](Self::adopt_child).
    pub fn drop_child(self: Handle<Self>, app: &mut App, child: Handle<PipelineOwner>) {
        debug_assert_eq!(app.get(child).debug_parent, Some(self));
        debug_assert!(app.get(self).children.contains(&child));
        debug_assert!(
            !app.get(self).debug_doing_child_layout,
            "Cannot modify child list after layout."
        );
        let children = &mut app.get_mut(self).children;
        let index = children
            .iter()
            .position(|&candidate| candidate == child)
            .expect("child was in the list");
        children.remove(index);
        if cfg!(debug_assertions) {
            app.get_mut(child).debug_parent = None;
        }
        if app.get(self).manifold.is_some() {
            child.detach(app);
        }
    }

    /// Calls `visitor` for each immediate child of this [`PipelineOwner`].
    pub fn visit_children(
        self: Handle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(Handle<PipelineOwner>),
    ) {
        for child in &app.get(self).children {
            visitor(*child);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use reveal_embedder::Size;
    use reveal_foundation::{App, AppCell};

    use super::*;
    use crate::box_::{RenderBox, RenderBoxData};
    use crate::object::{RenderHandle, RenderObject, RenderObjectData};

    struct TestLeaf {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        on_layout: Option<Rc<dyn Fn()>>,
    }

    impl RenderObject for TestLeaf {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            if let Some(on_layout) = self.get(app).on_layout.clone() {
                on_layout();
            }
            let size = self
                .render_box_data(app)
                .constraints
                .map(crate::box_::BoxConstraints::smallest)
                .unwrap_or(Size::ZERO);
            self.set_size(app, size);
        }
    }

    impl RenderBox for TestLeaf {
        crate::render_box_accessors!();
    }

    fn leaf(on_layout: Option<Rc<dyn Fn()>>) -> TestLeaf {
        TestLeaf {
            render_object: RenderObjectData::new(),
            render_box: RenderBoxData::new(),
            on_layout,
        }
    }

    /// `pipeline_owner_tree_test.dart`: parent's render objects are laid out
    /// before child's render objects.
    #[test]
    fn parent_pipeline_lays_out_before_child_pipeline() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Rc::new(RefCell::new(Vec::new()));

        let root_log = Rc::clone(&log);
        let root_node = RenderHandle::new_box(
            &mut app,
            leaf(Some(Rc::new(move || {
                root_log.borrow_mut().push("layout parent")
            }))),
        );
        let root = PipelineOwner::new(&mut app, None);
        root.set_root_node(&mut app, Some(root_node.as_object()));
        root_node.schedule_initial_layout(&mut app);

        let child_log = Rc::clone(&log);
        let child_node = RenderHandle::new_box(
            &mut app,
            leaf(Some(Rc::new(move || {
                child_log.borrow_mut().push("layout child")
            }))),
        );
        let child = PipelineOwner::new(&mut app, None);
        child.set_root_node(&mut app, Some(child_node.as_object()));
        child_node.schedule_initial_layout(&mut app);

        root.adopt_child(&mut app, child);
        root.flush_layout(&mut app);
        assert_eq!(*log.borrow(), ["layout parent", "layout child"]);
    }

    #[test]
    fn set_root_node_attaches_and_replacing_detaches() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = PipelineOwner::new(&mut app, None);
        let first = RenderHandle::new_box(&mut app, leaf(None));
        let second = RenderHandle::new_box(&mut app, leaf(None));
        owner.set_root_node(&mut app, Some(first.as_object()));
        assert!(first.attached(&app));
        assert_eq!(first.owner(&app), Some(owner));

        owner.set_root_node(&mut app, Some(second.as_object()));
        assert!(!first.attached(&app));
        assert!(second.attached(&app));
    }

    #[test]
    fn schedule_initial_layout_then_flush() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = PipelineOwner::new(&mut app, None);
        let node = RenderHandle::new_box(&mut app, leaf(None));
        owner.set_root_node(&mut app, Some(node.as_object()));
        node.schedule_initial_layout(&mut app);
        assert!(node.debug_needs_layout(&app));
        owner.flush_layout(&mut app);
        assert!(!node.debug_needs_layout(&app));
        assert_eq!(node.size(&app), Size::ZERO);
    }

    /// Counts the visual updates requested through it.
    struct CountingManifold {
        requests: Cell<u32>,
    }

    impl PipelineManifold for CountingManifold {
        fn request_visual_update(&self, _app: &mut App) {
            self.requests.set(self.requests.get() + 1);
        }
    }

    #[test]
    fn an_owner_without_a_callback_requests_visual_updates_through_its_manifold() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let manifold = Rc::new(CountingManifold {
            requests: Cell::new(0),
        });
        let root = PipelineOwner::new(&mut app, None);
        let adopted_before = PipelineOwner::new(&mut app, None);
        root.adopt_child(&mut app, adopted_before);

        adopted_before.request_visual_update(&mut app);
        assert_eq!(
            manifold.requests.get(),
            0,
            "unattached owners request nothing"
        );

        root.attach(&mut app, manifold.clone());
        root.request_visual_update(&mut app);
        adopted_before.request_visual_update(&mut app);
        assert_eq!(
            manifold.requests.get(),
            2,
            "attach reaches existing children"
        );

        let adopted_after = PipelineOwner::new(&mut app, None);
        root.adopt_child(&mut app, adopted_after);
        adopted_after.request_visual_update(&mut app);
        assert_eq!(manifold.requests.get(), 3, "adopt_child attaches the child");

        root.drop_child(&mut app, adopted_after);
        adopted_after.request_visual_update(&mut app);
        assert_eq!(manifold.requests.get(), 3, "drop_child detaches the child");

        root.detach(&mut app);
        adopted_before.request_visual_update(&mut app);
        assert_eq!(manifold.requests.get(), 3, "detach reaches the children");
    }

    #[test]
    fn a_callback_takes_precedence_over_the_manifold() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let manifold = Rc::new(CountingManifold {
            requests: Cell::new(0),
        });
        let callbacks = Rc::new(Cell::new(0));
        let owner = PipelineOwner::new(
            &mut app,
            Some(Listener::new({
                let callbacks = callbacks.clone();
                move |_app| callbacks.set(callbacks.get() + 1)
            })),
        );
        owner.attach(&mut app, manifold.clone());
        owner.request_visual_update(&mut app);
        assert_eq!((callbacks.get(), manifold.requests.get()), (1, 0));
    }
}

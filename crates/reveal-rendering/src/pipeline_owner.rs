//! Flutter counterpart: `rendering/object.dart` (`PipelineOwner`).
//!
//! Semantics callbacks, [`PipelineManifold`], and paint/compositing dirty
//! lists wait.

use reveal_foundation::{App, Handle, Listener};

use crate::object::AnyRenderObject;

/// The pipeline owner manages the rendering pipeline.
///
/// Flutter's counterpart is `PipelineOwner`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineOwner(Handle<PipelineOwnerData>);

struct PipelineOwnerData {
    on_need_visual_update: Option<Listener>,
    root_node: Option<AnyRenderObject>,
    nodes_needing_layout: Vec<AnyRenderObject>,
    should_merge_dirty_nodes: bool,
    debug_doing_layout: bool,
    debug_doing_child_layout: bool,
    children: Vec<PipelineOwner>,
    debug_parent: Option<PipelineOwner>,
}

impl PipelineOwner {
    /// Creates a pipeline owner.
    ///
    /// Typically created by the binding, but can be created separately to drive
    /// off-screen render objects through the rendering pipeline.
    pub fn new(app: &mut App, on_need_visual_update: Option<Listener>) -> PipelineOwner {
        PipelineOwner(app.create(PipelineOwnerData {
            on_need_visual_update,
            root_node: None,
            nodes_needing_layout: Vec::new(),
            should_merge_dirty_nodes: false,
            debug_doing_layout: false,
            debug_doing_child_layout: false,
            children: Vec::new(),
            debug_parent: None,
        }))
    }

    /// Calls [`on_need_visual_update`](Self::new) if one was provided.
    pub fn request_visual_update(self, app: &mut App) {
        let callback = app.get(self.0).on_need_visual_update.clone();
        if let Some(callback) = callback {
            callback.call(app);
        }
    }

    /// The unique object managed by this pipeline that has no parent.
    pub fn root_node(self, app: &App) -> Option<AnyRenderObject> {
        app.get(self.0).root_node
    }

    /// Sets the unique object managed by this pipeline that has no parent.
    pub fn set_root_node(self, app: &mut App, value: Option<AnyRenderObject>) {
        let current = app.get(self.0).root_node;
        if current == value {
            return;
        }
        if let Some(old) = current {
            old.detach(app);
        }
        app.get_mut(self.0).root_node = value;
        if let Some(new) = value {
            new.attach(app, self);
        }
    }

    /// Relayout boundaries which need to be laid out in the next
    /// [`flush_layout`](Self::flush_layout) pass.
    pub fn nodes_needing_layout(self, app: &App) -> Vec<AnyRenderObject> {
        app.get(self.0).nodes_needing_layout.clone()
    }

    pub(crate) fn add_node_needing_layout(self, app: &mut App, node: AnyRenderObject) {
        app.get_mut(self.0).nodes_needing_layout.push(node);
    }

    /// Whether this pipeline is currently in the layout phase.
    ///
    /// Always `false` when debug assertions are disabled.
    pub fn debug_doing_layout(self, app: &App) -> bool {
        if !cfg!(debug_assertions) {
            return false;
        }
        app.get(self.0).debug_doing_layout
    }

    /// Update the layout information for all dirty render objects.
    pub fn flush_layout(self, app: &mut App) {
        if cfg!(debug_assertions) {
            app.get_mut(self.0).debug_doing_layout = true;
        }
        while !app.get(self.0).nodes_needing_layout.is_empty() {
            debug_assert!(!app.get(self.0).should_merge_dirty_nodes);
            let mut dirty_nodes = std::mem::take(&mut app.get_mut(self.0).nodes_needing_layout);
            dirty_nodes.sort_by_key(|node| node.depth(app));
            let mut i = 0;
            while i < dirty_nodes.len() {
                if app.get(self.0).should_merge_dirty_nodes {
                    app.get_mut(self.0).should_merge_dirty_nodes = false;
                    if !app.get(self.0).nodes_needing_layout.is_empty() {
                        app.get_mut(self.0)
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
            app.get_mut(self.0).should_merge_dirty_nodes = false;
        }
        if cfg!(debug_assertions) {
            app.get_mut(self.0).debug_doing_child_layout = true;
        }
        let children = app.get(self.0).children.clone();
        for child in children {
            child.flush_layout(app);
        }
        debug_assert!(
            app.get(self.0).nodes_needing_layout.is_empty(),
            "Child PipelineOwners must not dirty nodes in their parent."
        );
        app.get_mut(self.0).should_merge_dirty_nodes = false;
        if cfg!(debug_assertions) {
            app.get_mut(self.0).debug_doing_layout = false;
            app.get_mut(self.0).debug_doing_child_layout = false;
        }
    }

    /// Adds `child` to this [`PipelineOwner`].
    pub fn adopt_child(self, app: &mut App, child: PipelineOwner) {
        debug_assert!(app.get(child.0).debug_parent.is_none());
        debug_assert!(!app.get(self.0).children.contains(&child));
        debug_assert!(
            !app.get(self.0).debug_doing_child_layout,
            "Cannot modify child list after layout."
        );
        app.get_mut(self.0).children.push(child);
        if cfg!(debug_assertions) {
            app.get_mut(child.0).debug_parent = Some(self);
        }
    }

    /// Removes a child [`PipelineOwner`] previously added via
    /// [`adopt_child`](Self::adopt_child).
    pub fn drop_child(self, app: &mut App, child: PipelineOwner) {
        debug_assert_eq!(app.get(child.0).debug_parent, Some(self));
        debug_assert!(app.get(self.0).children.contains(&child));
        debug_assert!(
            !app.get(self.0).debug_doing_child_layout,
            "Cannot modify child list after layout."
        );
        let children = &mut app.get_mut(self.0).children;
        let index = children
            .iter()
            .position(|&candidate| candidate == child)
            .expect("child was in the list");
        children.remove(index);
        if cfg!(debug_assertions) {
            app.get_mut(child.0).debug_parent = None;
        }
    }

    /// Calls `visitor` for each immediate child of this [`PipelineOwner`].
    pub fn visit_children(self, app: &App, visitor: &mut dyn FnMut(PipelineOwner)) {
        for child in &app.get(self.0).children {
            visitor(*child);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_embedder::Size;
    use reveal_foundation::App;

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
        let mut app = App::new();
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
        let mut app = App::new();
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
        let mut app = App::new();
        let owner = PipelineOwner::new(&mut app, None);
        let node = RenderHandle::new_box(&mut app, leaf(None));
        owner.set_root_node(&mut app, Some(node.as_object()));
        node.schedule_initial_layout(&mut app);
        assert!(node.debug_needs_layout(&app));
        owner.flush_layout(&mut app);
        assert!(!node.debug_needs_layout(&app));
        assert_eq!(node.size(&app), Size::ZERO);
    }
}

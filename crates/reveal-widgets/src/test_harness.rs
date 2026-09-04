//! A small render tree root for widget tests (the shape of `RawView`): a
//! `RenderTreeRootElement` whose render object is a repaint boundary with its own pipeline
//! owner, plus the frame pump. Tests across the crate mount widgets under it.

use std::rc::Rc;

use reveal_foundation::{App, Handle};
use reveal_rendering::{
    AnyRenderObject, BoxConstraints, CompositedLayer, PipelineOwner, RenderBox, RenderHandle,
    RenderObjectWithChildMixin, RenderRepaintBoundary,
};

use crate::*;

// ---- the test root: `RawView` in miniature ----

#[derive(Debug)]
pub(crate) struct TestRoot {
    pub(crate) child: WidgetRef,
}

impl RenderObjectWidget for TestRoot {
    type RenderObject = RenderRepaintBoundary;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderRepaintBoundary::new(app, None).as_object()
    }
}

pub(crate) struct TestRootElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    child: Option<AnyElement>,
    pipeline_owner: Option<Handle<PipelineOwner>>,
}

impl TestRootElement {
    pub(crate) fn create(app: &mut App, widget: WidgetRef) -> Handle<TestRootElement> {
        app.create(TestRootElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            child: None,
            pipeline_owner: None,
        })
    }

    fn update_child_from_widget(self: Handle<Self>, app: &mut App) {
        let child_widget = Self::widget_of(self.as_element().widget(app)).child.clone();
        let child = app.get(self).child;
        let child = self
            .as_element()
            .update_child(app, child, Some(child_widget), None);
        app.get_mut(self).child = child;
    }
}

impl RenderObjectElementWidget for TestRootElement {
    type Widget = TestRoot;

    fn widget_of(widget: &WidgetRef) -> &TestRoot {
        downcast_widget::<TestRoot>(&**widget).expect("the root holds a TestRoot")
    }
}

impl RenderObjectElement for TestRootElement {
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

impl RenderTreeRootElement for TestRootElement {}

impl Element for TestRootElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        if let Some(child) = app.get(self).child {
            visitor(child);
        }
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(app.get(self).child == Some(child));
        app.get_mut(self).child = None;
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        debug_assert!(parent.is_none());
        RenderObjectElement::mount(self, app, parent, new_slot);
        let owner = PipelineOwner::new(app, None);
        let render_object = RenderObjectElement::render_object(self, app);
        owner.set_root_node(app, Some(render_object));
        render_object.schedule_initial_layout(app);
        render_object.schedule_initial_paint(app, CompositedLayer::default());
        app.get_mut(self).pipeline_owner = Some(owner);
        self.update_child_from_widget(app);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        RenderObjectElement::update(self, app, new_widget);
        self.update_child_from_widget(app);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
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
        RenderTreeRootElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderTreeRootElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderTreeRootElement::detach_render_object(self, app);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        let root: RenderHandle<RenderRepaintBoundary> = self.typed_render_object(app);
        root.set_child(app, Some(child.as_box().expect("a box child")));
    }

    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        unreachable!("one child");
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        _child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        let root: RenderHandle<RenderRepaintBoundary> = self.typed_render_object(app);
        root.set_child(app, None);
    }
}

/// A mounted test tree: build owner, root element, and the frame pump.
pub(crate) struct Harness {
    pub(crate) owner: Handle<BuildOwner>,
    pub(crate) root: Handle<TestRootElement>,
}

impl Harness {
    pub(crate) fn mount(app: &mut App, child: WidgetRef) -> Harness {
        let owner = BuildOwner::new(app, None);
        let widget: WidgetRef = Rc::new(TestRootErased(TestRoot { child }));
        let root = TestRootElement::create(app, widget);
        root.as_element().assign_owner(app, owner);
        owner.build_scope(
            app,
            root.as_element(),
            Some(Box::new(move |app| {
                root.as_element().mount(app, None, None)
            })),
        );
        Harness { owner, root }
    }

    /// Replaces the root's child widget, the way a binding's `attachRootWidget` does.
    pub(crate) fn set_child(&self, app: &mut App, child: WidgetRef) {
        let widget: WidgetRef = Rc::new(TestRootErased(TestRoot { child }));
        let root = self.root;
        self.owner.build_scope(
            app,
            root.as_element(),
            Some(Box::new(move |app| root.as_element().update(app, widget))),
        );
    }

    /// A frame: build, layout, finalize.
    pub(crate) fn pump(&self, app: &mut App) {
        self.owner.build_scope(app, self.root.as_element(), None);
        let pipeline = app.get(self.root).pipeline_owner.expect("mounted");
        let render_root: RenderHandle<RenderRepaintBoundary> = self.root.typed_render_object(app);
        render_root.layout(
            app,
            BoxConstraints::new().max_width(300.0).max_height(200.0),
            false,
        );
        pipeline.flush_layout(app);
        pipeline.flush_paint(app);
        self.owner.finalize_tree(app);
    }

    pub(crate) fn render_root(&self, app: &App) -> RenderHandle<RenderRepaintBoundary> {
        self.root.typed_render_object(app)
    }
}

/// The erased `TestRoot`: the test root is not one of the public kinds.
#[derive(Debug)]
struct TestRootErased(TestRoot);

impl Widget for TestRootErased {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        TestRootElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.0
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<TestRoot>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

/// The `RenderPadding` directly under the test root, for assertions on a padded subtree.
pub(crate) fn padding_under_root(
    harness: &Harness,
    app: &App,
) -> RenderHandle<reveal_rendering::RenderPadding> {
    harness
        .render_root(app)
        .child(app)
        .expect("a child")
        .as_object()
        .downcast::<reveal_rendering::RenderPadding>(app)
        .expect("a RenderPadding")
}

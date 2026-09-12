//! A small render tree root for widget tests (the shape of `RawView`): a
//! `RenderTreeRootElement` whose render object is a `RenderView` over a fixed-size test view
//! with its own pipeline owner, plus the frame pump. Tests across the crate mount widgets
//! under it.

use std::rc::Rc;

use inset_embedder::{Picture, View as EmbedderView, ViewConstraints, ViewId, ViewMetrics};
use inset_foundation::{App, AppCell, Handle};
use inset_rendering::{
    AnyRenderObject, PipelineOwner, RenderHandle, RenderView, ViewConfiguration,
};

use crate::*;

// ---- the test root: `RawView` in miniature ----

/// The logical size every test tree is laid out in.
pub(crate) const VIEW_WIDTH: f64 = 300.0;
pub(crate) const VIEW_HEIGHT: f64 = 200.0;

/// A fixed-size view at pixel ratio 1 that drops what it is given to present.
struct TestView;

impl EmbedderView for TestView {
    fn id(&self) -> ViewId {
        ViewId(0)
    }

    fn metrics(&self) -> ViewMetrics {
        ViewMetrics {
            physical_size: [VIEW_WIDTH, VIEW_HEIGHT],
            physical_constraints: ViewConstraints::new(0.0, VIEW_WIDTH, 0.0, VIEW_HEIGHT),
            device_pixel_ratio: 1.0,
            ..ViewMetrics::default()
        }
    }

    fn present(&self, _picture: &Picture) {}
}

#[derive(Debug)]
pub(crate) struct TestRoot {
    pub(crate) child: WidgetRef,
}

impl RenderObjectWidget for TestRoot {
    type RenderObject = RenderView;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        let view = Rc::new(TestView);
        let configuration = ViewConfiguration::from_view(&*view);
        RenderView::new(app, None, Some(configuration), view).as_object()
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
        let render_view: RenderHandle<RenderView> = self.typed_render_object(app);
        owner.set_root_node(app, Some(render_view.as_object()));
        render_view.prepare_initial_frame(app);
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
        let root: RenderHandle<RenderView> = self.typed_render_object(app);
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
        let root: RenderHandle<RenderView> = self.typed_render_object(app);
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
        // Flutter's test binding attaches test trees to the binding's own build owner, which is
        // the one `GlobalKey` lookups consult.
        let owner = WidgetsBinding::instance(app).build_owner(app);
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

    /// A frame: build, layout, paint, finalize.
    pub(crate) fn pump(&self, app: &mut App) {
        self.owner.build_scope(app, self.root.as_element(), None);
        let pipeline = app.get(self.root).pipeline_owner.expect("mounted");
        pipeline.flush_layout(app);
        pipeline.flush_compositing_bits(app);
        pipeline.flush_paint(app);
        self.owner.finalize_tree(app);
    }

    pub(crate) fn render_root(&self, app: &App) -> RenderHandle<RenderView> {
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
) -> RenderHandle<inset_rendering::RenderPadding> {
    harness
        .render_root(app)
        .child(app)
        .expect("a child")
        .as_object()
        .downcast::<inset_rendering::RenderPadding>(app)
        .expect("a RenderPadding")
}

// ---- scroll test support ----

/// A [`ScrollContext`] for tests: its notification and storage contexts are the element it
/// was given, and its tickers come straight from the scheduler.
pub(crate) struct TestScrollContext {
    context: BuildContext,
    pub(crate) axis_direction: inset_painting::AxisDirection,
    pub(crate) device_pixel_ratio: f64,
    pub(crate) ignore_pointer: bool,
    pub(crate) can_drag: Vec<bool>,
    pub(crate) saved_offsets: Vec<f64>,
}

impl TestScrollContext {
    pub(crate) fn new(app: &mut App, context: BuildContext) -> Handle<TestScrollContext> {
        app.create(TestScrollContext {
            context,
            axis_direction: inset_painting::AxisDirection::Down,
            device_pixel_ratio: 1.0,
            ignore_pointer: false,
            can_drag: Vec::new(),
            saved_offsets: Vec::new(),
        })
    }
}

impl inset_scheduler::TickerProviderObject for TestScrollContext {
    fn create_ticker(
        self: Handle<Self>,
        app: &mut App,
        on_tick: inset_scheduler::TickerCallback,
    ) -> Handle<inset_scheduler::Ticker> {
        inset_scheduler::Ticker::new(app, on_tick)
    }
}

impl ScrollContext for Handle<TestScrollContext> {
    fn type_name(&self) -> &'static str {
        "TestScrollContext"
    }

    fn vsync(&self) -> Rc<dyn TickerProvider> {
        Rc::new(*self)
    }

    fn notification_context(&self, app: &mut App) -> Option<BuildContext> {
        let this = *self;
        Some(app.get(this).context)
    }

    fn storage_context(&self, app: &App) -> BuildContext {
        let this = *self;
        app.get(this).context
    }

    fn axis_direction(&self, app: &App) -> inset_painting::AxisDirection {
        let this = *self;
        app.get(this).axis_direction
    }

    fn device_pixel_ratio(&self, app: &App) -> f64 {
        let this = *self;
        app.get(this).device_pixel_ratio
    }

    fn set_ignore_pointer(&self, app: &mut App, value: bool) {
        let this = *self;
        app.get_mut(this).ignore_pointer = value;
    }

    fn set_can_drag(&self, app: &mut App, value: bool) {
        let this = *self;
        app.get_mut(this).can_drag.push(value);
    }

    fn save_offset(&self, app: &mut App, offset: f64) {
        let this = *self;
        app.get_mut(this).saved_offsets.push(offset);
    }
}

/// A mounted tree that records every `ScrollNotification` a descendant dispatches, plus the
/// [`TestScrollContext`] a scroll position under test can use.
pub(crate) struct ScrollHarness {
    #[allow(dead_code)]
    pub(crate) harness: Harness,
    pub(crate) context: Handle<TestScrollContext>,
    /// [`context`](Self::context) as the one `ScrollContext` every position under test shares.
    pub(crate) scroll_context: Rc<dyn ScrollContext>,
    pub(crate) notifications: Rc<std::cell::RefCell<Vec<String>>>,
}

/// The label a recorded notification is stored under.
fn notification_label(notification: &dyn ScrollNotification) -> String {
    let any = notification.as_any();
    if any.downcast_ref::<ScrollStartNotification>().is_some() {
        "start".to_string()
    } else if let Some(update) = any.downcast_ref::<ScrollUpdateNotification>() {
        format!("update {:?}", update.scroll_delta.unwrap_or_default())
    } else if let Some(overscroll) = any.downcast_ref::<OverscrollNotification>() {
        format!("overscroll {:?}", overscroll.overscroll)
    } else if any.downcast_ref::<ScrollEndNotification>().is_some() {
        "end".to_string()
    } else if let Some(user) = any.downcast_ref::<UserScrollNotification>() {
        format!("user {:?}", user.direction)
    } else {
        "unknown".to_string()
    }
}

/// Mounts the recorder tree and returns the scroll context under it.
pub(crate) fn mount_scroll_harness(app: &mut App) -> ScrollHarness {
    let notifications = Rc::new(std::cell::RefCell::new(Vec::new()));
    let recorded = notifications.clone();
    let tree = NotificationListener::<dyn ScrollNotification>::new(SizedBox::shrink())
        .on_notification(move |_app, notification: &dyn ScrollNotification| {
            recorded.borrow_mut().push(notification_label(notification));
            false
        })
        .into_widget();
    let harness = Harness::mount(app, tree);
    harness.pump(app);
    let listener = harness.root.as_element().children(app)[0];
    let inner = listener.children(app)[0];
    let context = TestScrollContext::new(app, inner);
    ScrollHarness {
        harness,
        context,
        scroll_context: Rc::new(context),
        notifications,
    }
}

// ---- the binding harness: a real `WidgetsBinding`, for trees that use a `GlobalKey` ----

/// A [`TestView`] behind a `PlatformRef`, so `run_widget` finds an implicit view.
struct BindingPlatform {
    view: inset_embedder::ViewRef,
}

impl inset_embedder::Platform for BindingPlatform {
    fn target_platform(&self) -> inset_embedder::TargetPlatform {
        inset_embedder::TargetPlatform::MacOS
    }

    fn request_frame(&self) {}

    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }

    fn wake_at(&self, _deadline: std::time::Instant) {}

    fn views(&self) -> Vec<inset_embedder::ViewRef> {
        vec![Rc::clone(&self.view)]
    }

    fn view(&self, id: ViewId) -> Option<inset_embedder::ViewRef> {
        (self.view.id() == id).then(|| Rc::clone(&self.view))
    }

    fn implicit_view(&self) -> Option<inset_embedder::ViewRef> {
        Some(Rc::clone(&self.view))
    }
}

/// An [`App`] whose platform has one [`VIEW_WIDTH`] x [`VIEW_HEIGHT`] view, for a tree that
/// needs the `WidgetsBinding` (a `GlobalKey` lookup, a post-frame callback, a timer).
pub(crate) fn binding_cell() -> Rc<AppCell> {
    let platform: inset_embedder::PlatformRef = Rc::new(BindingPlatform {
        view: Rc::new(TestView),
    });
    AppCell::with_platform(platform)
}

/// Mounts `child` under the platform's view and runs the first frame.
pub(crate) fn binding_mount(cell: &AppCell, child: WidgetRef) {
    {
        let mut app = cell.borrow_mut();
        let view = app
            .platform()
            .implicit_view()
            .expect("an app from binding_cell");
        crate::binding::run_widget(&mut app, View::new(view, child).into_widget());
    }
    // `run_app` attaches the root widget on the next timer turn, as Dart's `Timer.run` does.
    cell.elapse(std::time::Duration::ZERO);
    binding_pump(&mut cell.borrow_mut(), std::time::Duration::ZERO);
}

/// Runs one frame at `at`, then drains the microtasks it queued.
pub(crate) fn binding_pump(app: &mut App, at: std::time::Duration) {
    inset_scheduler::SchedulerBinding::handle_begin_frame(app, Some(at));
    app.drain_microtasks();
    inset_scheduler::SchedulerBinding::handle_draw_frame(app);
    app.drain_microtasks();
}

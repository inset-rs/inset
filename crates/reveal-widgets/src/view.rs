//! Flutter counterpart: `widgets/view.dart` (`View`, `RawView`, the view scopes).
//!
//! `View` wraps its child in `MediaQuery::from_view`, a `FocusTraversalGroup` parented on the
//! root scope, and a `FocusScope` over its own scope node. `ViewCollection` / `ViewAnchor`
//! wait.

use std::fmt;
use std::rc::Rc;

use reveal_embedder::ViewRef;
use reveal_foundation::{App, Handle, ValueKey};
use reveal_rendering::{AnyRenderObject, PipelineOwner, RenderHandle, RenderView, RendererBinding};

use crate::framework::{
    AnyElement, BuildContext, Element, ElementBase, ElementData, InheritedWidget, IntoWidget,
    KeyRef, RenderObjectElement, RenderObjectElementData, RenderObjectElementWidget,
    RenderObjectWidget, RenderTreeRootElement, Slot, State, StateData, StatefulWidget,
    StatelessWidget, Widget, WidgetKind, WidgetRef, downcast_widget,
};
use crate::widgets::focus_manager::{FocusManager, FocusNodeLeaf, FocusScopeNode};
use crate::widgets::focus_scope::FocusScope;
use crate::widgets::focus_traversal::{
    AnyFocusTraversalPolicy, FocusTraversalGroup, FocusTraversalPolicy, ReadingOrderTraversalPolicy,
};
use crate::widgets::media_query::MediaQuery;

/// Bootstraps a render tree that is rendered into the provided `FlutterView`.
///
/// The content rendered into that view is determined by the provided
/// [`child`](Self::child). Descendants within the same `LookupBoundary` can look up the view
/// they are rendered into via [`View::of`] and [`View::maybe_of`].
///
/// The provided [`child`](Self::child) is wrapped in a `MediaQuery` constructed from the given
/// view, a `FocusScope`, and a [`RawView`] widget.
///
/// For most use cases, using `MediaQuery.of`, or its associated "...Of" methods are a more
/// appropriate way of obtaining the information that a `FlutterView` exposes. For example,
/// using `MediaQuery.sizeOf` will expose the _logical_ device size (`MediaQueryData.size`)
/// rather than the physical size (`FlutterView.physicalSize`). Similarly, while
/// `FlutterView.padding` conveys the information from the operating system, the
/// `MediaQueryData.padding` attribute (obtained from `MediaQuery.paddingOf`) further adjusts
/// this information to be aware of the context of the widget; e.g. the `Scaffold` widget
/// adjusts the values for its various children.
///
/// Each `FlutterView` can be associated with at most one [`View`] widget in the widget tree.
/// Two or more [`View`] widgets configured with the same `FlutterView` must never exist within
/// the same widget tree at the same time. This limitation is enforced by a `GlobalObjectKey`
/// that derives its identity from the [`view`](Self::view) provided to this widget.
///
/// Since the [`View`] widget bootstraps its own independent render tree, neither it nor any of
/// its descendants will insert a `RenderObject` into an existing render tree. Therefore, the
/// [`View`] widget can only be used in those parts of the widget tree where it is not required
/// to participate in the construction of the surrounding render tree. In other words, the
/// widget may only be used in a non-rendering zone of the widget tree (see
/// `WidgetsBinding` for a definition of rendering and non-rendering zones).
pub struct View {
    pub key: Option<KeyRef>,
    /// The `FlutterView` into which [`child`](Self::child) is drawn.
    pub view: ViewRef,
    /// The widget below this widget in the tree, which will be drawn into the
    /// [`view`](Self::view).
    pub child: WidgetRef,
}

impl View {
    /// Create a [`View`] widget to bootstrap a render tree that is rendered into the provided
    /// `FlutterView`.
    pub fn new<K>(view: ViewRef, child: impl IntoWidget<K>) -> View {
        View {
            key: None,
            view,
            child: child.into_widget(),
        }
    }

    /// Dart `View(key:)`.
    pub fn key(mut self, key: KeyRef) -> View {
        self.key = Some(key);
        self
    }

    /// Returns the `FlutterView` that the provided `context` will render into.
    ///
    /// Returns `None` if the `context` is not associated with a `FlutterView`.
    ///
    /// The method creates a dependency on the `context`, which will be informed when the
    /// identity of the `FlutterView` changes (i.e. the `context` is moved to render into a
    /// different `FlutterView` then before). The dependency is not triggered when the
    /// properties on the `FlutterView` itself change.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<ViewRef> {
        context
            .depend_on_inherited_widget_of_exact_type::<ViewScope>(app)
            .map(|scope| Rc::clone(&scope.view))
    }

    /// Returns the `FlutterView` that the provided `context` will render into.
    ///
    /// Throws if the `context` is not associated with a `FlutterView`.
    ///
    /// The method creates a dependency on the `context`, which will be informed when the
    /// identity of the `FlutterView` changes (i.e. the `context` is moved to render into a
    /// different `FlutterView` then before). The dependency is not triggered when the
    /// properties on the `FlutterView` itself change.
    pub fn of(app: &mut App, context: BuildContext) -> ViewRef {
        View::maybe_of(app, context).expect(
            "View.of() was called with a context that does not contain a View widget. No View \
             widget ancestor could be found starting from the context that was passed to \
             View.of(). This usually means that the provided context is not associated with a \
             View.",
        )
    }

    /// Returns the `PipelineOwner` parent to which a child `View` should attach its
    /// `PipelineOwner` to.
    ///
    /// If `context` has a `View` ancestor, it returns the `PipelineOwner` that manages the
    /// render tree of that view. If there is no `View` ancestor,
    /// `RendererBinding.rootPipelineOwner` is returned instead.
    pub fn pipeline_owner_of(app: &mut App, context: BuildContext) -> Handle<PipelineOwner> {
        let scoped = context
            .depend_on_inherited_widget_of_exact_type::<PipelineOwnerScope>(app)
            .map(|scope| scope.pipeline_owner);
        scoped.unwrap_or_else(|| RendererBinding::instance(app).root_pipeline_owner(app))
    }
}

impl fmt::Debug for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("View")
            .field("view", &self.view.id())
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for View {
    type State = ViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ViewState {
        ViewState {
            state: StateData::new(),
            scope_node: None,
            policy: None,
        }
    }
}

/// The state of a [`View`]. Flutter's `_ViewState` also observes view focus events, which wait
/// with the platform's view-focus channel.
pub struct ViewState {
    state: StateData<View>,
    scope_node: Option<Handle<FocusScopeNode>>,
    policy: Option<AnyFocusTraversalPolicy>,
}

impl ViewState {
    /// The focus scope node this view's subtree is scoped by.
    fn scope_node(self: Handle<Self>, app: &App) -> Handle<FocusScopeNode> {
        app.get(self).scope_node.expect("created in init_state")
    }
}

impl State for ViewState {
    type Widget = View;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let scope_node = FocusScopeNode::new(app);
        scope_node.set_debug_label(app, Some("View Scope".to_string()));
        let policy = ReadingOrderTraversalPolicy::new(app).as_policy();
        let this = app.get_mut(self);
        this.scope_node = Some(scope_node);
        this.policy = Some(policy);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.scope_node(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let (view, child) = {
            let widget = self.widget(app);
            (Rc::clone(&widget.view), widget.child.clone())
        };
        let scope_node = self.scope_node(app);
        let policy = app.get(self).policy.expect("created in init_state");
        let root_scope = FocusManager::instance(app).root_scope(app);
        // Attach this view's focus subtree directly to the root scope rather than nesting it
        // under an enclosing view's scope (which happens when a view is rendered inside another
        // via a `ViewAnchor`). Each view is an independent focus root: nesting would otherwise
        // make an ancestor view report `has_focus` when a descendant view is focused.
        let scoped = FocusTraversalGroup::new(
            FocusScope::with_external_focus_node(child, scope_node).include_semantics(false),
        )
        .policy(policy)
        .parent_node(root_scope.as_node());
        let content = MediaQuery::from_view(None, Rc::clone(&view), scoped);
        RawView::new(view, content).into_widget()
    }
}

/// The lower level workhorse widget for [`View`] that bootstraps a render tree for a
/// `FlutterView`.
///
/// This widget does not have any dependencies on other widgets which is why it can be used
/// as a root widget for the widget tree.
///
/// The [`View`] widget wraps this widget in some additional widgets to expose information
/// about the `FlutterView` to the widget tree. It is not necessary to have a [`View`] widget
/// as the root of the tree, but only [`RawView`] widgets at the roots of the tree will not
/// have access to `MediaQuery` and other information exposed by the [`View`] widget.
pub struct RawView {
    pub key: Option<KeyRef>,
    /// The `FlutterView` into which [`child`](Self::child) is drawn.
    pub view: ViewRef,
    /// The widget below this widget in the tree, which will be drawn into the
    /// [`view`](Self::view).
    pub child: WidgetRef,
}

impl RawView {
    /// Creates a [`RawView`] widget to bootstrap a render tree that is rendered into the
    /// provided `FlutterView`.
    pub fn new<K>(view: ViewRef, child: impl IntoWidget<K>) -> RawView {
        RawView {
            key: None,
            view,
            child: child.into_widget(),
        }
    }

    /// Dart `RawView(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawView {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for RawView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawView")
            .field("view", &self.view.id())
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for RawView {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let view = Rc::clone(&self.view);
        let child = self.child.clone();
        RawViewInternal::new(
            Rc::clone(&self.view),
            Rc::new(
                move |_app: &mut App, _context: BuildContext, owner: Handle<PipelineOwner>| {
                    let scoped = PipelineOwnerScope::new(owner, child.clone());
                    ViewScope::new(Rc::clone(&view), scoped).into_widget()
                },
            ),
        )
        .into_widget()
    }
}

/// Dart's `_RawViewContentBuilder`.
type RawViewContentBuilder = Rc<dyn Fn(&mut App, BuildContext, Handle<PipelineOwner>) -> WidgetRef>;

/// Dart's `_RawViewInternal`: the render object widget behind [`RawView`], keyed by its view
/// so two views never share an element.
pub struct RawViewInternal {
    key: KeyRef,
    view: ViewRef,
    builder: RawViewContentBuilder,
}

impl RawViewInternal {
    fn new(view: ViewRef, builder: RawViewContentBuilder) -> RawViewInternal {
        RawViewInternal {
            key: Rc::new(ValueKey::new(view.id().0)),
            view,
            builder,
        }
    }

    fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }
}

impl fmt::Debug for RawViewInternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawViewInternal")
            .field("view", &self.view.id())
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for RawViewInternal {
    type RenderObject = RenderView;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderView::new(app, None, None, Rc::clone(&self.view)).as_object()
    }
}

impl Widget for RawViewInternal {
    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        RawViewElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<RawViewInternal>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

/// Dart's `_RawViewElement`: the root of a render tree, with its own `PipelineOwner`.
pub struct RawViewElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    pipeline_owner: Handle<PipelineOwner>,
    child: Option<AnyElement>,
    /// Is `None` if the view is currently not attached.
    parent_pipeline_owner: Option<Handle<PipelineOwner>>,
}

impl RawViewElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<RawViewElement> {
        // Semantics callbacks wait with accessibility.
        let pipeline_owner = PipelineOwner::new(app, None);
        app.create(RawViewElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            pipeline_owner,
            child: None,
            parent_pipeline_owner: None,
        })
    }

    /// The `RenderView` at the root of this view's render tree.
    pub fn render_view(self: Handle<Self>, app: &App) -> RenderHandle<RenderView> {
        self.typed_render_object(app)
    }

    /// The pipeline owner that lays out and paints this view's render tree.
    pub fn pipeline_owner(self: Handle<Self>, app: &App) -> Handle<PipelineOwner> {
        app.get(self).pipeline_owner
    }

    fn update_child(self: Handle<Self>, app: &mut App) {
        let builder = Self::widget_of(self.as_element().widget(app))
            .builder
            .clone();
        let owner = self.pipeline_owner(app);
        let built = builder(app, self.as_element(), owner);
        let child = app.get(self).child;
        let child = self
            .as_element()
            .update_child(app, child, Some(built), None);
        app.get_mut(self).child = child;
    }

    fn attach_view(
        self: Handle<Self>,
        app: &mut App,
        parent_pipeline_owner: Option<Handle<PipelineOwner>>,
    ) {
        debug_assert!(app.get(self).parent_pipeline_owner.is_none());
        let parent_pipeline_owner = parent_pipeline_owner
            .unwrap_or_else(|| View::pipeline_owner_of(app, self.as_element()));
        let owner = self.pipeline_owner(app);
        parent_pipeline_owner.adopt_child(app, owner);
        let render_view = self.render_view(app);
        RendererBinding::instance(app).add_render_view(app, render_view);
        app.get_mut(self).parent_pipeline_owner = Some(parent_pipeline_owner);
    }

    fn detach_view(self: Handle<Self>, app: &mut App) {
        if let Some(parent_pipeline_owner) = app.get(self).parent_pipeline_owner {
            let render_view = self.render_view(app);
            RendererBinding::instance(app).remove_render_view(app, render_view);
            let owner = self.pipeline_owner(app);
            parent_pipeline_owner.drop_child(app, owner);
            app.get_mut(self).parent_pipeline_owner = None;
        }
    }
}

impl RenderObjectElementWidget for RawViewElement {
    type Widget = RawViewInternal;

    fn widget_of(widget: &WidgetRef) -> &RawViewInternal {
        downcast_widget::<RawViewInternal>(&**widget).expect("a RawViewElement holds its widget")
    }
}

impl RenderObjectElement for RawViewElement {
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

impl RenderTreeRootElement for RawViewElement {}

impl Element for RawViewElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        false
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
        RenderObjectElement::mount(self, app, parent, new_slot);
        let owner = self.pipeline_owner(app);
        debug_assert!(owner.root_node(app).is_none());
        let render_view = self.render_view(app);
        owner.set_root_node(app, Some(render_view.as_object()));
        self.attach_view(app, None);
        self.update_child(app);
        render_view.prepare_initial_frame(app);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        RenderObjectElement::update(self, app, new_widget);
        self.update_child(app);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
        self.update_child(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        ElementBase::did_change_dependencies(self, app);
        if app.get(self).parent_pipeline_owner.is_none() {
            return;
        }
        let new_parent_pipeline_owner = View::pipeline_owner_of(app, self.as_element());
        if Some(new_parent_pipeline_owner) != app.get(self).parent_pipeline_owner {
            self.detach_view(app);
            self.attach_view(app, Some(new_parent_pipeline_owner));
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        ElementBase::activate(self, app);
        let owner = self.pipeline_owner(app);
        debug_assert!(owner.root_node(app).is_none());
        let render_view = self.render_view(app);
        owner.set_root_node(app, Some(render_view.as_object()));
        self.attach_view(app, None);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        self.detach_view(app);
        let owner = self.pipeline_owner(app);
        debug_assert!(owner.root_node(app) == Some(self.render_view(app).as_object()));
        owner.set_root_node(app, None); // To satisfy the assert in the super class.
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        // Dart disposes its pipeline owner here; ours has no resources beyond the arena
        // slot, which goes with the element.
        let owner = self.pipeline_owner(app);
        RenderObjectElement::unmount(self, app);
        app.destroy(owner);
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
        slot: Option<Slot>,
    ) {
        debug_assert!(slot.is_none());
        let render_view = self.render_view(app);
        render_view.set_child(app, Some(child.as_box().expect("a view holds a box")));
    }

    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        debug_assert!(false, "a view has one child");
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        debug_assert!(slot.is_none());
        let render_view = self.render_view(app);
        debug_assert!(render_view.child(app).map(|current| current.as_object()) == Some(child));
        render_view.set_child(app, None);
    }
}

/// Dart's `_ViewScope`: the view a subtree renders into.
pub struct ViewScope {
    pub view: ViewRef,
    pub child: WidgetRef,
}

impl ViewScope {
    /// Creates a `ViewScope`; Dart's arguments are all required.
    pub fn new<K>(view: ViewRef, child: impl IntoWidget<K>) -> ViewScope {
        ViewScope {
            view,
            child: child.into_widget(),
        }
    }
}

impl fmt::Debug for ViewScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewScope")
            .field("view", &self.view.id())
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for ViewScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ViewScope) -> bool {
        !std::ptr::addr_eq(Rc::as_ptr(&self.view), Rc::as_ptr(&old_widget.view))
    }
}

/// Dart's `_PipelineOwnerScope`: the pipeline owner a subtree's views attach to.
#[derive(Debug)]
pub struct PipelineOwnerScope {
    pub pipeline_owner: Handle<PipelineOwner>,
    pub child: WidgetRef,
}

impl PipelineOwnerScope {
    /// Creates a `PipelineOwnerScope`; Dart's arguments are all required.
    pub fn new<K>(
        pipeline_owner: Handle<PipelineOwner>,
        child: impl IntoWidget<K>,
    ) -> PipelineOwnerScope {
        PipelineOwnerScope {
            pipeline_owner,
            child: child.into_widget(),
        }
    }
}

impl InheritedWidget for PipelineOwnerScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &PipelineOwnerScope) -> bool {
        self.pipeline_owner != old_widget.pipeline_owner
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use reveal_scheduler::SchedulerBinding;

    use crate::framework::{GlobalKey, IntoWidget};
    use crate::test_harness::{binding_app, binding_mount, binding_pump};
    use crate::widgets::basic::SizedBox;

    /// A view's render tree lives under the view's own `PipelineOwner`, which has no visual
    /// update callback: its requests must reach the binding through the manifold.
    #[test]
    fn a_dirty_render_object_under_a_view_schedules_a_frame() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(
            &mut app,
            SizedBox::new()
                .key(key.clone())
                .width(10.0)
                .height(10.0)
                .into_widget(),
        );
        for _ in 0..5 {
            binding_pump(&mut app, std::time::Duration::from_millis(16));
        }
        assert!(
            !SchedulerBinding::has_scheduled_frame(&mut app),
            "the tree is idle before the probe"
        );
        let render_object = key
            .current_context(&mut app)
            .expect("mounted")
            .find_render_object(&app)
            .expect("a render object");

        render_object.mark_needs_paint(&mut app);
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));

        binding_pump(&mut app, std::time::Duration::from_millis(32));
        assert!(!SchedulerBinding::has_scheduled_frame(&mut app));
        render_object.mark_needs_layout(&mut app);
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));
    }
}

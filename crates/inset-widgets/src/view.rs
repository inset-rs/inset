//! Flutter counterpart: `widgets/view.dart` (`View`, `RawView`, the view scopes).
//!
//! `View` wraps its child in `MediaQuery::from_view`, a `FocusTraversalGroup` parented on the
//! root scope, and a `FocusScope` over its own scope node. `ViewCollection` and `ViewAnchor`
//! place further views beside or under a tree.

use std::any::Any;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

use inset_embedder::{ViewFocusDirection, ViewFocusEvent, ViewFocusState, ViewRef};
use inset_foundation::{App, Handle, ListenableObject, Listener, ValueKey};
use inset_rendering::{AnyRenderObject, PipelineOwner, RenderHandle, RenderView, RendererBinding};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};
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
            view_has_focus: false,
            observer: None,
            scope_listener: None,
        }
    }
}

/// The state of a [`View`], including synchronization with native view focus.
pub struct ViewState {
    state: StateData<View>,
    scope_node: Option<Handle<FocusScopeNode>>,
    policy: Option<AnyFocusTraversalPolicy>,
    /// Whether the native view owns focus, independently of its widget subtree.
    view_has_focus: bool,
    /// Retains the observer identity for removal on disposal.
    observer: Option<WidgetsBindingObserverRef>,
    /// Retains the bound callback identity for removal on disposal.
    scope_listener: Option<Listener>,
}

impl ViewState {
    /// The focus scope node this view's subtree is scoped by.
    fn scope_node(self: Handle<Self>, app: &App) -> Handle<FocusScopeNode> {
        app.get(self).scope_node.expect("created in init_state")
    }

    /// Requests native focus when the framework focuses a descendant of this view.
    fn scope_focus_change_listener(self: Handle<Self>, app: &mut App) {
        let has_focus = self.scope_node(app).has_focus(app);
        if app.get(self).view_has_focus == has_focus || !has_focus {
            return;
        }
        app.platform().request_view_focus_change(
            self.widget(app).view.id(),
            ViewFocusState::Focused,
            ViewFocusDirection::Forward,
        );
    }
}

impl WidgetsBindingObserverObject for ViewState {
    fn did_change_view_focus(self: Handle<Self>, app: &mut App, event: ViewFocusEvent) {
        let view_id = self.widget(app).view.id();
        app.get_mut(self).view_has_focus = match event.state {
            ViewFocusState::Focused => event.view_id == view_id,
            ViewFocusState::Unfocused => false,
        };
        if event.view_id != view_id {
            return;
        }
        match event.state {
            ViewFocusState::Focused => {
                let scope = self.scope_node(app).as_node();
                let policy = app.get(self).policy.expect("created in init_state");
                let next_focus = match event.direction {
                    ViewFocusDirection::Forward => {
                        policy.find_first_focus(app, scope, true).unwrap_or(scope)
                    }
                    ViewFocusDirection::Backward => policy.find_last_focus(app, scope, true),
                    ViewFocusDirection::Undefined => scope,
                };
                next_focus.request_focus(app, None);
            }
            ViewFocusState::Unfocused => {
                FocusManager::instance(app)
                    .root_scope(app)
                    .request_scope_focus(app);
            }
        }
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
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        WidgetsBinding::instance(app).add_observer(app, observer.clone());
        app.get_mut(self).observer = Some(observer);
        let listener = Listener::handle_method(self, ViewState::scope_focus_change_listener);
        scope_node.add_listener(app, listener.clone());
        app.get_mut(self).scope_listener = Some(listener);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get_mut(self).observer.take() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
        if let Some(listener) = app.get_mut(self).scope_listener.take() {
            self.scope_node(app).remove_listener(app, &listener);
        }
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

/// Dart's `_MultiChildComponentWidget`: views that each bootstrap a render tree of their
/// own, and at most one child that takes part in the surrounding render tree. Subclasses
/// choose what to make public.
pub struct MultiChildComponentWidget {
    key: Option<KeyRef>,
    views: Vec<WidgetRef>,
    child: Option<WidgetRef>,
}

impl MultiChildComponentWidget {
    fn new(key: Option<KeyRef>, views: Vec<WidgetRef>, child: Option<WidgetRef>) -> Self {
        MultiChildComponentWidget { key, views, child }
    }

    fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }
}

impl fmt::Debug for MultiChildComponentWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiChildComponentWidget")
            .field("views", &self.views.len())
            .field("has_child", &self.child.is_some())
            .finish()
    }
}

impl Widget for MultiChildComponentWidget {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        MultiChildComponentElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<MultiChildComponentWidget>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

/// A collection of sibling [`View`]s.
///
/// This widget can only be used in places were a [`View`] widget is allowed, i.e. in a
/// non-rendering zone of the widget tree. In practical terms, it can be used at the root of
/// the widget tree outside of any [`View`] widget, as a child of a [`ViewAnchor`], or in the
/// `view` slot of a [`ViewAnchor`]. It cannot be used as a normal child of a widget that
/// expects a `RenderObject` as its child.
pub struct ViewCollection {
    pub key: Option<KeyRef>,
    /// The [`View`] widgets that are part of this collection.
    pub views: Vec<WidgetRef>,
}

impl ViewCollection {
    pub fn new(views: Vec<WidgetRef>) -> ViewCollection {
        ViewCollection { key: None, views }
    }

    /// Dart `ViewCollection(key:)`.
    pub fn key(mut self, key: KeyRef) -> ViewCollection {
        self.key = Some(key);
        self
    }

    pub fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }
}

impl fmt::Debug for ViewCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewCollection")
            .field("views", &self.views.len())
            .finish()
    }
}

impl Widget for ViewCollection {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        MultiChildComponentElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<ViewCollection>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

/// Decorates a `child` widget with a side [`View`].
///
/// This widget must have a [`View`] ancestor, into which the `child` widget is rendered. The
/// `view` widget is rendered into a view of its own, while the `child` renders into the
/// enclosing one.
#[derive(Debug)]
pub struct ViewAnchor {
    pub key: Option<KeyRef>,
    /// The widget that defines the view anchored to this widget.
    pub view: Option<WidgetRef>,
    /// The widget below this widget in the tree, rendered into the enclosing view.
    pub child: WidgetRef,
}

impl ViewAnchor {
    pub fn new<K>(child: impl IntoWidget<K>) -> ViewAnchor {
        ViewAnchor {
            key: None,
            view: None,
            child: child.into_widget(),
        }
    }

    /// Dart `ViewAnchor(key:)`.
    pub fn key(mut self, key: KeyRef) -> ViewAnchor {
        self.key = Some(key);
        self
    }

    /// Dart `ViewAnchor(view:)`.
    pub fn view<K>(mut self, view: impl IntoWidget<K>) -> ViewAnchor {
        self.view = Some(view.into_widget());
        self
    }
}

impl StatelessWidget for ViewAnchor {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        // `LookupBoundary` around the view waits (`## Deferred`).
        MultiChildComponentWidget::new(
            None,
            self.view.iter().cloned().collect(),
            Some(self.child.clone()),
        )
        .into_widget()
    }
}

thread_local! {
    /// Dart's `_MultiChildComponentElement._viewSlot`: one object every view child is
    /// slotted with, told apart from a render slot by identity.
    static VIEW_SLOT: Rc<dyn Any> = Rc::new(());
}

fn view_slot() -> Slot {
    Slot::Custom(VIEW_SLOT.with(Rc::clone))
}

fn is_view_slot(slot: Option<&Slot>) -> bool {
    matches!(slot, Some(Slot::Custom(slot)) if VIEW_SLOT.with(|view_slot| Rc::ptr_eq(view_slot, slot)))
}

/// Dart's `_MultiChildComponentElement`.
pub struct MultiChildComponentElement {
    element: ElementData,
    view_elements: Vec<AnyElement>,
    forgotten_view_elements: HashSet<AnyElement>,
    child_element: Option<AnyElement>,
}

impl MultiChildComponentElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<MultiChildComponentElement> {
        app.create(MultiChildComponentElement {
            element: ElementData::new(widget),
            view_elements: Vec::new(),
            forgotten_view_elements: HashSet::new(),
            child_element: None,
        })
    }

    /// The views and the child the widget carries, whichever of the two widget types it is.
    fn parts(widget: &WidgetRef) -> (Vec<WidgetRef>, Option<WidgetRef>) {
        if let Some(collection) = downcast_widget::<ViewCollection>(&**widget) {
            return (collection.views.clone(), None);
        }
        let widget = downcast_widget::<MultiChildComponentWidget>(&**widget)
            .expect("a MultiChildComponentElement holds a ViewCollection or a ViewAnchor's widget");
        (widget.views.clone(), widget.child.clone())
    }

    fn debug_assert_children(self: Handle<Self>, app: &App) -> bool {
        let (views, child) = Self::parts(self.as_element().widget(app));
        let this = app.get(self);
        // Each view widget must have a corresponding element.
        debug_assert!(this.view_elements.len() == views.len());
        // Iff there is a child widget, it must have a corresponding element.
        debug_assert!(this.child_element.is_none() == child.is_none());
        // The child element is not also a view element.
        debug_assert!(
            this.child_element
                .is_none_or(|child| !this.view_elements.contains(&child))
        );
        true
    }

    /// Dart's `_debugCheckMustAttachRenderObject`: in the [`ViewCollection`] configuration,
    /// no ancestor may expect a render object in this element's slot.
    fn debug_check_must_attach_render_object(
        self: Handle<Self>,
        app: &App,
        slot: Option<&Slot>,
    ) -> bool {
        if !cfg!(debug_assertions) || Self::parts(self.as_element().widget(app)).1.is_some() {
            return true;
        }
        let mut has_ancestor_render_object_element = false;
        let mut ancestor_wants_render_object = true;
        self.as_element()
            .visit_ancestor_elements(app, &mut |ancestor| {
                if !ancestor.debug_expects_render_object_for_slot(app, slot) {
                    ancestor_wants_render_object = false;
                    return false;
                }
                if ancestor.is_render_object_element() {
                    has_ancestor_render_object_element = true;
                    return false;
                }
                true
            });
        debug_assert!(
            !(has_ancestor_render_object_element && ancestor_wants_render_object),
            "A ViewCollection cannot be inserted into a slot its ancestor expects a render \
             object in; move it into the view property of a ViewAnchor widget or to the root \
             of the widget tree."
        );
        true
    }

    /// The body of `performRebuild`: the child in this element's own slot, the views in the
    /// view slot.
    fn update_children(self: Handle<Self>, app: &mut App) {
        let (views, child) = Self::parts(self.as_element().widget(app));
        let slot = self.as_element().slot(app);
        let child_element = app.get(self).child_element;
        let child_element = self
            .as_element()
            .update_child(app, child_element, child, slot);
        app.get_mut(self).child_element = child_element;

        let old_views = app.get(self).view_elements.clone();
        let is_forgotten =
            |app: &App, child: AnyElement| app.get(self).forgotten_view_elements.contains(&child);
        let slots: Vec<Option<Slot>> = views.iter().map(|_| Some(view_slot())).collect();
        let view_elements = self.as_element().update_children(
            app,
            &old_views,
            &views,
            Some(&is_forgotten),
            Some(&slots),
        );
        let this = app.get_mut(self);
        this.view_elements = view_elements;
        this.forgotten_view_elements.clear();
    }
}

impl Element for MultiChildComponentElement {
    crate::element_accessors!();

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        ElementBase::attach_render_object(self, app, new_slot.clone());
        debug_assert!(self.debug_check_must_attach_render_object(app, new_slot.as_ref()));
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ElementBase::mount(self, app, parent, new_slot.clone());
        debug_assert!(self.debug_check_must_attach_render_object(app, new_slot.as_ref()));
        debug_assert!(app.get(self).view_elements.is_empty());
        debug_assert!(app.get(self).child_element.is_none());
        self.update_children(app);
        ElementBase::perform_rebuild(self, app);
        debug_assert!(self.debug_assert_children(app));
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        ElementBase::update_slot(self, app, new_slot.clone());
        debug_assert!(self.debug_check_must_attach_render_object(app, new_slot.as_ref()));
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        // Cannot switch from ViewAnchor config to ViewCollection config.
        debug_assert!(
            Self::parts(&new_widget).1.is_none()
                == Self::parts(self.as_element().widget(app)).1.is_none()
        );
        ElementBase::update(self, app, new_widget);
        self.as_element().rebuild(app, true);
        debug_assert!(self.debug_assert_children(app));
    }

    fn debug_expects_render_object_for_slot(
        self: Handle<Self>,
        _app: &App,
        slot: Option<&Slot>,
    ) -> bool {
        !is_view_slot(slot)
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        self.update_children(app);
        ElementBase::perform_rebuild(self, app); // clears the dirty flag
        debug_assert!(self.debug_assert_children(app));
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        let this = app.get_mut(self);
        if this.child_element == Some(child) {
            this.child_element = None;
        } else {
            debug_assert!(this.view_elements.contains(&child));
            debug_assert!(!this.forgotten_view_elements.contains(&child));
            this.forgotten_view_elements.insert(child);
        }
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        let this = app.get(self);
        if let Some(child) = this.child_element {
            visitor(child);
        }
        for child in &this.view_elements {
            if !this.forgotten_view_elements.contains(child) {
                visitor(*child);
            }
        }
    }

    fn debug_doing_build(self: Handle<Self>, _app: &App) -> bool {
        false // This element does not have a concept of "building".
    }

    fn render_object_attaching_child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        app.get(self).child_element
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use inset_scheduler::SchedulerBinding;

    use crate::binding::WidgetsBinding;
    use crate::framework::{GlobalKey, IntoWidget};
    use crate::test_harness::{binding_cell, binding_mount, binding_pump};
    use crate::widgets::basic::SizedBox;

    /// A view's render tree lives under the view's own `PipelineOwner`, which has no visual
    /// update callback: its requests must reach the binding through the manifold.
    #[test]
    fn a_dirty_render_object_under_a_view_schedules_a_frame() {
        let cell = binding_cell();
        let app = cell.borrow();
        let key = Rc::new(GlobalKey::new());
        drop(app);
        binding_mount(
            &cell,
            SizedBox::new()
                .key(key.clone())
                .width(10.0)
                .height(10.0)
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
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

    /// Flutter view_test.dart: platform focus parks on the root and restores the view's history.
    #[test]
    fn platform_focus_enters_parks_and_restores_the_view() {
        use crate::{Focus, FocusManager, FocusNode, FocusNodeLeaf, Row};
        use inset_embedder::{
            ViewFocusDirection as Direction, ViewFocusEvent, ViewFocusState as State,
        };
        let cell = binding_cell();
        let (first, last) = {
            let mut app = cell.borrow_mut();
            (FocusNode::new(&mut app), FocusNode::new(&mut app))
        };
        binding_mount(
            &cell,
            crate::Directionality::new(
                inset_embedder::TextDirection::Ltr,
                Row::new().children(vec![
                    Focus::new(SizedBox::new().width(20.0).height(20.0))
                        .focus_node(first.as_node())
                        .into_widget(),
                    Focus::new(SizedBox::new().width(20.0).height(20.0))
                        .focus_node(last.as_node())
                        .into_widget(),
                ]),
            )
            .into_widget(),
        );
        let event = |state, direction| {
            let mut app = cell.borrow_mut();
            let view_id = app.platform().implicit_view().unwrap().id();
            WidgetsBinding::instance(&mut app).handle_view_focus_changed(
                &mut app,
                ViewFocusEvent {
                    view_id,
                    state,
                    direction,
                },
            );
            drop(app);
            cell.checkpoint();
        };
        event(State::Focused, Direction::Forward);
        assert!(first.has_primary_focus(&cell.borrow()));
        event(State::Unfocused, Direction::Undefined);
        {
            let mut app = cell.borrow_mut();
            assert!(
                FocusManager::instance(&mut app)
                    .root_scope(&app)
                    .has_primary_focus(&app)
            );
        }
        event(State::Focused, Direction::Undefined);
        assert!(first.has_primary_focus(&cell.borrow()));
        event(State::Unfocused, Direction::Undefined);
        event(State::Focused, Direction::Backward);
        assert!(last.has_primary_focus(&cell.borrow()));
        event(State::Unfocused, Direction::Undefined);
        event(State::Focused, Direction::Undefined);
        assert!(last.has_primary_focus(&cell.borrow()));
    }

    /// A host view that counts its presents, for trees with more than one view.
    struct CountingView {
        id: inset_embedder::ViewId,
        presented: Rc<std::cell::Cell<u32>>,
    }

    impl inset_embedder::View for CountingView {
        fn id(&self) -> inset_embedder::ViewId {
            self.id
        }

        fn metrics(&self) -> inset_embedder::ViewMetrics {
            inset_embedder::ViewMetrics {
                physical_size: [400.0, 300.0],
                physical_constraints: inset_embedder::ViewConstraints::tight(400.0, 300.0),
                device_pixel_ratio: 2.0,
                ..inset_embedder::ViewMetrics::default()
            }
        }

        fn present(&self, _picture: std::sync::Arc<inset_embedder::Picture>) {
            self.presented.set(self.presented.get() + 1);
        }
    }

    fn counting_view(id: u64) -> (inset_embedder::ViewRef, Rc<std::cell::Cell<u32>>) {
        let presented = Rc::new(std::cell::Cell::new(0));
        let view: inset_embedder::ViewRef = Rc::new(CountingView {
            id: inset_embedder::ViewId(id),
            presented: Rc::clone(&presented),
        });
        (view, presented)
    }

    /// Attaches `root` as the whole tree and runs the first frame.
    fn mount_root(cell: &inset_foundation::AppCell, root: crate::WidgetRef) {
        crate::binding::run_widget(&mut cell.borrow_mut(), root);
        cell.elapse(std::time::Duration::ZERO);
        binding_pump(&mut cell.borrow_mut(), std::time::Duration::ZERO);
    }

    #[test]
    fn a_view_collection_bootstraps_one_render_tree_per_view() {
        use inset_rendering::RendererBinding;

        let cell = binding_cell();
        let (first, first_presented) = counting_view(1);
        let (second, second_presented) = counting_view(2);
        mount_root(
            &cell,
            super::ViewCollection::new(vec![
                super::View::new(first, SizedBox::new()).into_widget(),
                super::View::new(second, SizedBox::new()).into_widget(),
            ])
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        assert_eq!(binding.render_views(&app).len(), 2);
        assert_eq!(first_presented.get(), 1);
        assert_eq!(second_presented.get(), 1);
    }

    #[test]
    fn a_view_anchor_renders_its_child_in_place_and_its_view_beside_it() {
        use inset_rendering::RendererBinding;

        let cell = binding_cell();
        let (enclosing, enclosing_presented) = counting_view(1);
        let (side, side_presented) = counting_view(2);
        mount_root(
            &cell,
            super::View::new(
                enclosing,
                super::ViewAnchor::new(SizedBox::new().width(10.0).height(10.0))
                    .view(super::View::new(side, SizedBox::new())),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let binding = RendererBinding::instance(&mut app);
        assert_eq!(binding.render_views(&app).len(), 2);
        assert_eq!(enclosing_presented.get(), 1);
        assert_eq!(side_presented.get(), 1);
    }

    struct FakeWindow {
        view: inset_embedder::ViewRef,
    }

    impl inset_embedder::HostWindow for FakeWindow {
        fn view(&self) -> inset_embedder::ViewRef {
            Rc::clone(&self.view)
        }

        fn frame(&self) -> inset_embedder::Rect {
            inset_embedder::Rect::from_ltwh(1.0, 2.0, 3.0, 4.0)
        }

        fn set_frame(&self, _frame: inset_embedder::Rect, _animate: Option<std::time::Duration>) {}

        fn set_title(&self, _title: &str) {}

        fn show(&self) {}

        fn hide(&self) {}

        fn close(&self) {}

        fn native_handle(&self) -> Option<inset_embedder::RawWindowHandle> {
            None
        }

        fn set_close_requested(&self, _handler: Option<Rc<dyn Fn()>>) {}
    }

    #[test]
    fn a_window_hands_itself_to_its_subtree() {
        use crate::widgets::basic::Builder;
        use crate::window::{Window, WindowScope};

        let cell = binding_cell();
        let (view, presented) = counting_view(7);
        let window: inset_embedder::WindowRef = Rc::new(FakeWindow { view });
        let seen = Rc::new(std::cell::Cell::new(None));
        let probe = {
            let seen = Rc::clone(&seen);
            Builder::new(move |app, context| {
                seen.set(Some(WindowScope::of(app, context).frame()));
                SizedBox::new().into_widget()
            })
        };
        mount_root(
            &cell,
            super::ViewCollection::new(vec![Window::new(window, probe).into_widget()])
                .into_widget(),
        );
        assert_eq!(
            seen.get(),
            Some(inset_embedder::Rect::from_ltwh(1.0, 2.0, 3.0, 4.0))
        );
        assert_eq!(presented.get(), 1);
    }
}

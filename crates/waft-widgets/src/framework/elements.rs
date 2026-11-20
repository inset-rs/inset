//! The concrete element classes: `ComponentElement` and its `StatelessElement` /
//! `StatefulElement` / `ProxyElement` (`InheritedElement`, `ParentDataElement`), and
//! `RenderObjectElement` with `LeafRenderObjectElement` / `SingleChildRenderObjectElement`
//! and the `RenderTreeRootElement` base.
//!
//! Flutter's base classes are traits holding the shared bodies; each concrete element is a
//! struct generic over its widget type, so the widget is reached without a cast.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use waft_foundation::{App, Handle, HandleId};
use waft_rendering::{
    AnyRenderObject, ContainerRenderObjectMixin, ErasedRenderObject, RenderHandle,
    RenderObjectWithChildMixin,
};

use super::element::{
    AnyElement, Element, ElementBase, ElementData, ElementLifecycle, IndexedSlot, Slot,
    debug_check_owner_build_target_exists,
};
use super::state::{AnyState, State, StateLifecycle};
use super::widget::{
    AnyInheritedWidget, AnyParentDataWidget, LeafRenderObjectWidget, MultiChildRenderObjectWidget,
    ParentDataWidget, RenderObjectWidget, SingleChildRenderObjectWidget, WidgetRef,
    downcast_widget,
};

// ---------------------------------------------------------------------------------------------
// ComponentElement

/// The fields of Dart's `ComponentElement`.
#[derive(Default)]
pub struct ComponentElementData {
    pub(crate) child: Option<AnyElement>,
    pub(crate) debug_doing_build: bool,
}

/// An [`Element`] that composes other [`Element`]s.
///
/// Rather than creating a `RenderObject` directly, a [`ComponentElement`] creates
/// `RenderObject`s indirectly by creating other [`Element`]s.
///
/// Contrast with `RenderObjectElement`.
pub trait ComponentElement: Element {
    /// The child slot and build flag, held under the field `component`.
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData;

    /// See [`component_data`](Self::component_data).
    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData;

    /// Subclasses should override this function to actually call the appropriate `build`
    /// function (e.g., `StatelessWidget::build` or `State::build`) for their widget.
    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef;

    /// Runs [`build`](Self::build) inside [`App::entity_track`] and binds watches.
    #[inline(never)]
    fn build_tracked(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let element = self.as_element();
        let (built, tracked) = app.entity_track(|app| self.build(app));
        element.bind_entity_watches(app, tracked);
        built
    }

    /// The child this element built, if any.
    fn child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        self.component_data(app).child
    }

    /// `ComponentElement.mount`: the base mount, then the first build.
    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ElementBase::mount(self, app, parent, new_slot);
        debug_assert!(self.child(app).is_none());
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Active);
        self.first_build(app);
        debug_assert!(self.child(app).is_some());
    }

    /// `ComponentElement._firstBuild`.
    #[inline(always)]
    fn first_build(self: Handle<Self>, app: &mut App) {
        ElementBase::rebuild(self, app, false); // This eventually calls performRebuild.
    }

    /// Calls the [`build`](Self::build) method and then updates the widget tree.
    ///
    /// Called automatically during [`ComponentElement::mount`](Self::mount) to generate the
    /// first build, and by `rebuild` when the element needs updating.
    #[inline(always)]
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        if cfg!(debug_assertions) {
            self.component_data_mut(app).debug_doing_build = true;
        }
        let built = self.build_tracked(app);
        if cfg!(debug_assertions) {
            self.component_data_mut(app).debug_doing_build = false;
        }
        ElementBase::perform_rebuild(self, app); // clears the "dirty" flag
        let child = self.child(app);
        let slot = self.as_element().slot(app);
        let child = self
            .as_element()
            .update_child(app, child, Some(built), slot);
        debug_assert!(child.is_some());
        self.component_data_mut(app).child = child;
    }

    /// `ComponentElement.visitChildren`.
    fn visit_children_component(
        self: Handle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyElement),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child);
        }
    }

    /// `ComponentElement.forgetChild`.
    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(self.child(app) == Some(child));
        self.component_data_mut(app).child = None;
    }
}

/// `impl Element` for a component element whose accessors are `element_accessors!()` and
/// whose base bodies are the component ones.
macro_rules! component_element_overrides {
    () => {
        fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
            ComponentElement::visit_children_component(self, app, visitor);
        }

        fn render_object_attaching_child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
            ComponentElement::child(self, app)
        }

        fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
            ComponentElement::forget_child(self, app, child);
        }

        fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
            self.component_data(app).debug_doing_build
        }
    };
}
pub(crate) use component_element_overrides;

// ---------------------------------------------------------------------------------------------
// StatelessElement

/// An [`Element`] that uses a [`StatelessWidget`] as its configuration.
pub struct StatelessElement {
    element: ElementData,
    component: ComponentElementData,
}

impl StatelessElement {
    /// Creates an element that uses the given widget as its configuration.
    pub fn new(app: &mut App, widget: WidgetRef) -> Handle<Self> {
        debug_assert!(widget.as_stateless().is_some());
        app.create(StatelessElement {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
        })
        .into_tree_node(app)
    }
}

impl ComponentElement for StatelessElement {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.as_element().widget(app).clone();
        let stateless = widget
            .as_stateless()
            .expect("a StatelessElement holds a StatelessWidget");
        stateless.build(app, self.as_element())
    }
}

impl Element for StatelessElement {
    crate::element_accessors!();
    component_element_overrides!();

    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ElementBase::update(self, app, new_widget);
        ElementBase::rebuild(self, app, true);
    }

    #[inline(always)]
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }
}

// ---------------------------------------------------------------------------------------------
// StatefulElement

/// An [`Element`] that uses a [`StatefulWidget`] as its configuration.
pub struct StatefulElement {
    element: ElementData,
    component: ComponentElementData,
    state: Option<AnyState>,
    did_change_dependencies: bool,
}

impl StatefulElement {
    /// Creates an element that uses the given widget as its configuration, with the `state`
    /// the widget created.
    pub(crate) fn new(app: &mut App, widget: WidgetRef, state: AnyState) -> Handle<Self> {
        let this = app
            .create(StatefulElement {
                element: ElementData::new(widget.clone()),
                component: ComponentElementData::default(),
                state: Some(state),
                did_change_dependencies: false,
            })
            .into_tree_node(app);
        debug_assert!(state.element(app).is_none());
        state.set_element(app, Some(this));
        state.set_widget(app, widget);
        debug_assert!(state.debug_lifecycle_state(app) == StateLifecycle::Created);
        this
    }

    /// The [`State`] instance associated with this location in the tree, when it is an `S`.
    ///
    /// There is a one-to-one relationship between [`State`] objects and the
    /// [`StatefulElement`] objects that hold them. The [`State`] objects are created by
    /// [`StatefulElement`] in `mount`.
    pub fn state<S: State>(self: Handle<Self>, app: &App) -> Option<Handle<S>> {
        let state = app.get(self).state.expect("the element is unmounted");
        (state.type_id() == TypeId::of::<S>()).then(|| Handle::from_id(state.id()))
    }

    fn any_state(self: Handle<Self>, app: &App) -> AnyState {
        app.get(self).state.expect("the element is unmounted")
    }
}

impl ComponentElement for StatefulElement {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let state = self.any_state(app);
        state.build(app, self.as_element())
    }

    #[inline(always)]
    fn first_build(self: Handle<Self>, app: &mut App) {
        let state = self.any_state(app);
        debug_assert!(state.debug_lifecycle_state(app) == StateLifecycle::Created);
        state.init_state(app);
        if cfg!(debug_assertions) {
            state.set_debug_lifecycle_state(app, StateLifecycle::Initialized);
        }
        state.did_change_dependencies(app);
        if cfg!(debug_assertions) {
            state.set_debug_lifecycle_state(app, StateLifecycle::Ready);
        }
        ElementBase::rebuild(self, app, false);
    }

    #[inline(always)]
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        if app.get(self).did_change_dependencies {
            self.any_state(app).did_change_dependencies(app);
            app.get_mut(self).did_change_dependencies = false;
        }
        if cfg!(debug_assertions) {
            self.component_data_mut(app).debug_doing_build = true;
        }
        let built = self.build_tracked(app);
        if cfg!(debug_assertions) {
            self.component_data_mut(app).debug_doing_build = false;
        }
        ElementBase::perform_rebuild(self, app); // clears the "dirty" flag
        let child = self.child(app);
        let slot = self.as_element().slot(app);
        let child = self
            .as_element()
            .update_child(app, child, Some(built), slot);
        debug_assert!(child.is_some());
        self.component_data_mut(app).child = child;
    }
}

impl Element for StatefulElement {
    crate::element_accessors!();
    component_element_overrides!();

    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        ElementBase::update(self, app, new_widget.clone());
        let state = self.any_state(app);
        state.set_widget(app, new_widget);
        state.did_update_widget(app, &old_widget);
        ElementBase::rebuild(self, app, true);
    }

    #[inline(always)]
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        ElementBase::activate(self, app);
        self.any_state(app).activate(app);
        // Since the State could have observed the deactivate() and thus disposed of
        // resources allocated in the build method, we have to rebuild the widget
        // so that its State can reallocate its resources.
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Active); // otherwise markNeedsBuild is a no-op
        self.as_element().mark_needs_build(app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        self.any_state(app).deactivate(app);
        ElementBase::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        ElementBase::unmount(self, app);
        let state = self.any_state(app);
        state.dispose(app);
        if cfg!(debug_assertions) {
            state.set_debug_lifecycle_state(app, StateLifecycle::Defunct);
        }
        state.set_element(app, None);
        app.get_mut(self).state = None;
        app.release_tree_node(state.id());
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        ElementBase::did_change_dependencies(self, app);
        app.get_mut(self).did_change_dependencies = true;
    }

    fn state_handle_id(self: Handle<Self>, app: &App) -> Option<(TypeId, HandleId)> {
        app.get(self)
            .state
            .map(|state| (state.type_id(), state.id()))
    }
}

// ---------------------------------------------------------------------------------------------
// ProxyElement

/// An [`Element`] that uses a `ProxyWidget` as its configuration.
pub trait ProxyElement: ComponentElement {
    /// The proxied child widget.
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef;

    /// Called during build when the widget has changed.
    ///
    /// By default, calls [`notify_clients`](Self::notify_clients). Subclasses may override
    /// this method to avoid calling [`notify_clients`](Self::notify_clients) unnecessarily
    /// (e.g. if the old and new widgets are equivalent).
    fn updated(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        self.notify_clients(app, old_widget);
    }

    /// Notify other objects that the widget associated with this element has changed.
    ///
    /// Called during [`update`](Element::update) (via [`updated`](Self::updated)) after
    /// changing the widget associated with this element but before rebuilding this element.
    fn notify_clients(self: Handle<Self>, app: &mut App, old_widget: WidgetRef);

    /// `ProxyElement.update`.
    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        debug_assert!(!Rc::ptr_eq(&old_widget, &new_widget));
        ElementBase::update(self, app, new_widget);
        self.updated(app, old_widget);
        ElementBase::rebuild(self, app, true);
    }
}

// ---------------------------------------------------------------------------------------------
// InheritedElement

/// An [`Element`] that uses an [`InheritedWidget`] as its configuration.
pub struct InheritedElement {
    element: ElementData,
    component: ComponentElementData,
    dependents: HashMap<AnyElement, Option<Rc<dyn Any>>>,
}

impl InheritedElement {
    /// Creates an element that uses the given widget as its configuration.
    pub fn new(app: &mut App, widget: WidgetRef) -> Handle<Self> {
        debug_assert!(widget.as_inherited().is_some());
        app.create(InheritedElement {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            dependents: HashMap::new(),
        })
        .into_tree_node(app)
    }

    /// The widget, as an inherited widget.
    pub fn widget(self: Handle<Self>, app: &App) -> &dyn AnyInheritedWidget {
        self.as_element()
            .widget(app)
            .as_inherited()
            .expect("an InheritedElement holds an InheritedWidget")
    }

    /// Returns the dependencies value recorded for `dependent` with
    /// [`set_dependencies`](Self::set_dependencies).
    ///
    /// Each dependent element is mapped to a single object value which represents how the
    /// element depends on this [`InheritedElement`]. This value is `None` by default and by
    /// default dependent elements are rebuilt unconditionally.
    pub fn get_dependencies(
        self: Handle<Self>,
        app: &App,
        dependent: AnyElement,
    ) -> Option<Rc<dyn Any>> {
        app.get(self).dependents.get(&dependent).cloned().flatten()
    }

    /// Sets the value returned by [`get_dependencies`](Self::get_dependencies) value for
    /// `dependent`.
    ///
    /// Each dependent element is mapped to a single object value which represents how the
    /// element depends on this [`InheritedElement`]. The `update_dependencies` method sets
    /// this value to `None` by default so that dependent elements are rebuilt unconditionally.
    pub fn set_dependencies(
        self: Handle<Self>,
        app: &mut App,
        dependent: AnyElement,
        value: Option<Rc<dyn Any>>,
    ) {
        app.get_mut(self).dependents.insert(dependent, value);
    }

    /// Called by [`notify_clients`](ProxyElement::notify_clients) for each dependent.
    ///
    /// Calls `dependent.did_change_dependencies()` by default.
    ///
    /// Subclasses can override this method to selectively call
    /// `did_change_dependencies` based on the value of
    /// [`get_dependencies`](Self::get_dependencies).
    pub fn notify_dependent(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &WidgetRef,
        dependent: AnyElement,
    ) {
        let _ = (self, old_widget);
        dependent.did_change_dependencies(app);
    }
}

impl ComponentElement for InheritedElement {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        self.proxied_child(app)
    }
}

impl ProxyElement for InheritedElement {
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
        self.widget(app).child().clone()
    }

    fn updated(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        let should_notify = self.widget(app).update_should_notify(&*old_widget);
        if should_notify {
            self.notify_clients(app, old_widget);
        }
    }

    fn notify_clients(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        debug_assert!(debug_check_owner_build_target_exists(
            self.as_element(),
            app,
            "notifyClients"
        ));
        let dependents: Vec<AnyElement> = app.get(self).dependents.keys().copied().collect();
        for dependent in dependents {
            if cfg!(debug_assertions) {
                // check that it really is our descendant
                let mut ancestor = dependent.parent(app);
                while let Some(current) = ancestor
                    && current != self.as_element()
                {
                    ancestor = current.parent(app);
                }
                debug_assert!(ancestor == Some(self.as_element()));
                // check that it really depends on us
                debug_assert!(dependent.does_depend_on_inherited_element(app, self.as_element()));
            }
            self.notify_dependent(app, &old_widget, dependent);
        }
    }
}

impl Element for InheritedElement {
    crate::element_accessors!();
    component_element_overrides!();

    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ProxyElement::update(self, app, new_widget);
    }

    #[inline(always)]
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }

    fn update_inheritance(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Active);
        let incoming = self
            .as_element()
            .parent(app)
            .and_then(|parent| parent.inherited_elements(app));
        let mut widgets: HashMap<TypeId, AnyElement> = incoming
            .map(|incoming| (*incoming).clone())
            .unwrap_or_default();
        let widget_type = self.as_element().widget(app).widget_type();
        widgets.insert(widget_type, self.as_element());
        self.as_element()
            .set_inherited_elements(app, Some(Rc::new(widgets)));
    }

    fn debug_deactivated(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).dependents.is_empty());
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Inactive);
    }

    const IS_INHERITED_ELEMENT: bool = true;

    fn update_dependencies(
        self: Handle<Self>,
        app: &mut App,
        dependent: AnyElement,
        aspect: Option<Rc<dyn Any>>,
    ) {
        let _ = aspect;
        self.set_dependencies(app, dependent, None);
    }

    fn remove_dependent(self: Handle<Self>, app: &mut App, dependent: AnyElement) {
        app.get_mut(self).dependents.remove(&dependent);
    }
}

// ---------------------------------------------------------------------------------------------
// ParentDataElement

/// An [`Element`] that uses a [`ParentDataWidget`] as its configuration.
pub struct ParentDataElement {
    element: ElementData,
    component: ComponentElementData,
}

impl ParentDataElement {
    /// Creates an element that uses the given widget as its configuration.
    pub fn new(app: &mut App, widget: WidgetRef) -> Handle<Self> {
        debug_assert!(widget.as_parent_data().is_some());
        app.create(ParentDataElement {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
        })
        .into_tree_node(app)
    }

    /// The widget, as a parent data widget.
    pub fn widget(self: Handle<Self>, app: &App) -> &dyn AnyParentDataWidget {
        self.as_element()
            .widget(app)
            .as_parent_data()
            .expect("a ParentDataElement holds a ParentDataWidget")
    }

    fn apply_parent_data_to_descendants(self: Handle<Self>, app: &mut App) {
        let mut child = self.as_element().render_object_attaching_child(app);
        while let Some(current) = child {
            if current.is_render_object_element() {
                current.update_parent_data(app, self.as_element());
                return;
            }
            child = current.render_object_attaching_child(app);
        }
    }

    /// Calls `ParentDataWidget::apply_parent_data` on the given widget, passing it the
    /// `RenderObject` whose parent data this element is ultimately responsible for.
    ///
    /// This allows a render object's `RenderObject::parent_data` to be modified without
    /// triggering a build. This is generally ill-advised, but makes sense in situations such
    /// as the following:
    ///
    ///  * Build and layout are currently under way, but the `ParentData` in question does
    ///    not affect layout, and the value to be applied could not be determined before build
    ///    and layout (e.g. it depends on the layout of a descendant).
    ///
    ///  * Paint is currently under way, but the `ParentData` in question does not affect
    ///    layout or paint, and the value to be applied could not be determined before paint
    ///    (e.g. it depends on the compositing phase).
    ///
    /// In either case, the next build is expected to cause this element to be configured
    /// with the given new widget (or a widget with equivalent data).
    ///
    /// Only [`ParentDataWidget`]s that return true for
    /// `ParentDataWidget::debug_can_apply_out_of_turn` can be applied this way.
    ///
    /// The new widget must have the same child as the current widget.
    pub fn apply_widget_out_of_turn<W: ParentDataWidget>(
        self: Handle<Self>,
        app: &mut App,
        new_widget: &W,
    ) {
        debug_assert!(new_widget.debug_can_apply_out_of_turn());
        debug_assert!(Rc::ptr_eq(new_widget.child(), self.widget(app).child()));
        let mut child = self.as_element().render_object_attaching_child(app);
        while let Some(current) = child {
            if current.is_render_object_element() {
                let render_object = current
                    .render_object(app)
                    .expect("a mounted RenderObjectElement has a render object");
                new_widget.apply_parent_data(app, render_object);
                return;
            }
            child = current.render_object_attaching_child(app);
        }
    }
}

impl ComponentElement for ParentDataElement {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        self.proxied_child(app)
    }
}

impl ProxyElement for ParentDataElement {
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
        self.widget(app).child().clone()
    }

    fn notify_clients(self: Handle<Self>, app: &mut App, _old_widget: WidgetRef) {
        self.apply_parent_data_to_descendants(app);
    }
}

impl Element for ParentDataElement {
    crate::element_accessors!();
    component_element_overrides!();

    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ProxyElement::update(self, app, new_widget);
    }

    #[inline(always)]
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }

    const IS_PARENT_DATA_ELEMENT: bool = true;

    fn apply_parent_data(
        self: Handle<Self>,
        app: &mut App,
        render_object: AnyRenderObject,
    ) -> bool {
        // `debugIsValidRenderObject`: the render object must carry this widget's parent data.
        let widget = self.as_element().widget(app).clone();
        let parent_data_widget = widget
            .as_parent_data()
            .expect("a ParentDataElement holds a ParentDataWidget");
        let valid = parent_data_widget.debug_is_valid_render_object(app, render_object);
        debug_assert!(
            valid,
            "Incorrect use of ParentDataWidget. The ParentDataWidget {:?} wants to apply \
             ParentData of type {} to a RenderObject which has been set up to accept ParentData \
             of an incompatible type. Usually, this means that the widget has the wrong \
             ancestor RenderObjectWidget.",
            parent_data_widget,
            parent_data_widget.parent_data_type_name()
        );
        if valid {
            parent_data_widget.apply_parent_data(app, render_object);
        }
        valid
    }
}

// ---------------------------------------------------------------------------------------------
// RenderObjectElement

/// The fields of Dart's `RenderObjectElement`.
#[derive(Default)]
pub struct RenderObjectElementData {
    render_object: Option<AnyRenderObject>,
    ancestor_render_object_element: Option<AnyElement>,
    debug_doing_build: bool,
}

impl RenderObjectElementData {
    /// The render object this element created, once mounted.
    pub fn render_object(&self) -> Option<AnyRenderObject> {
        self.render_object
    }

    /// Whether this element is currently creating or updating its render object.
    pub fn debug_doing_build(&self) -> bool {
        self.debug_doing_build
    }
}

/// An [`Element`] that uses a [`RenderObjectWidget`] as its configuration.
///
/// [`RenderObjectElement`] objects have an associated `RenderObject` widget in the render
/// tree, which handles concrete operations like laying out, painting, and hit testing.
///
/// Contrast with [`ComponentElement`].
///
/// For details on the lifecycle of an element, see the discussion at [`Element`].
///
/// ## Writing a RenderObjectElement subclass
///
/// There are three common child models used by most `RenderObject`s:
///
/// * Leaf render objects, with no children: the [`LeafRenderObjectElement`] class handles
///   this case.
///
/// * A single child: the [`SingleChildRenderObjectElement`] class handles this case.
///
/// * A linked list of children: the `MultiChildRenderObjectElement` class handles this case
///   (deferred).
pub trait RenderObjectElement: Element {
    /// The render object slot, held under the field `render_object_element`.
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData;

    /// See [`render_object_element_data`](Self::render_object_element_data).
    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData;

    /// The underlying `RenderObject` for this element.
    fn render_object(self: Handle<Self>, app: &App) -> AnyRenderObject {
        self.render_object_element_data(app)
            .render_object
            .expect("unmounted")
    }

    /// The typed render object.
    fn typed_render_object(
        self: Handle<Self>,
        app: &App,
    ) -> RenderHandle<
        <<Self as RenderObjectElementWidget>::Widget as RenderObjectWidget>::RenderObject,
    >
    where
        Self: RenderObjectElementWidget,
    {
        RenderObjectElement::render_object(self, app)
            .downcast(app)
            .expect("the render object this widget created")
    }

    fn find_ancestor_render_object_element(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        let slot = self.as_element().slot(app);
        let mut ancestor = self.as_element().parent(app);
        while let Some(current) = ancestor
            && !current.is_render_object_element()
        {
            // In debug mode we check whether the ancestor accepts RenderObjects to produce a
            // better error message in attachRenderObject. In release mode, we assume only
            // RenderObjectElements are in the walk.
            if cfg!(debug_assertions)
                && !current.debug_expects_render_object_for_slot(app, slot.as_ref())
            {
                return None;
            }
            ancestor = current.parent(app);
        }
        if cfg!(debug_assertions)
            && let Some(current) = ancestor
            && !current.debug_expects_render_object_for_slot(app, slot.as_ref())
        {
            return None;
        }
        ancestor
    }

    fn find_ancestor_parent_data_elements(self: Handle<Self>, app: &App) -> Vec<AnyElement> {
        let mut ancestor = self.as_element().parent(app);
        let mut result = Vec::new();
        while let Some(current) = ancestor
            && !current.is_render_object_element()
        {
            if current.is_parent_data_element() {
                result.push(current);
            }
            ancestor = current.parent(app);
        }
        result
    }

    /// `RenderObjectElement.mount`: the base mount, create the render object, attach it.
    #[inline(always)]
    fn mount(self: Handle<Self>, app: &mut App, parent: Option<AnyElement>, new_slot: Option<Slot>)
    where
        Self: RenderObjectElementWidget,
    {
        ElementBase::mount(self, app, parent, new_slot.clone());
        if cfg!(debug_assertions) {
            self.render_object_element_data_mut(app).debug_doing_build = true;
        }
        let render_object = {
            let widget = self.as_element().widget(app).clone();
            let typed = Self::widget_of(&widget);
            typed.create_render_object(app, self.as_element())
        };
        self.render_object_element_data_mut(app).render_object = Some(render_object);
        if cfg!(debug_assertions) {
            self.render_object_element_data_mut(app).debug_doing_build = false;
        }
        debug_assert!(self.as_element().slot(app) == new_slot);
        self.as_element().attach_render_object(app, new_slot);
        ElementBase::perform_rebuild(self, app); // clears the "dirty" flag
    }

    /// `RenderObjectElement.update`.
    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef)
    where
        Self: RenderObjectElementWidget,
    {
        ElementBase::update(self, app, new_widget);
        RenderObjectElement::perform_rebuild(self, app); // calls widget.updateRenderObject()
    }

    /// `RenderObjectElement._performRebuild`: push the widget's configuration.
    #[inline(always)]
    fn perform_rebuild(self: Handle<Self>, app: &mut App)
    where
        Self: RenderObjectElementWidget,
    {
        if cfg!(debug_assertions) {
            self.render_object_element_data_mut(app).debug_doing_build = true;
        }
        let widget = self.as_element().widget(app).clone();
        let render_object = self.typed_render_object(app);
        Self::widget_of(&widget).update_render_object(app, self.as_element(), render_object);
        if cfg!(debug_assertions) {
            self.render_object_element_data_mut(app).debug_doing_build = false;
        }
        ElementBase::perform_rebuild(self, app); // clears the "dirty" flag
    }

    /// `RenderObjectElement.deactivate`.
    fn deactivate(self: Handle<Self>, app: &mut App) {
        ElementBase::deactivate(self, app);
        debug_assert!(
            !RenderObjectElement::render_object(self, app).attached(app),
            "A RenderObject was still attached when attempting to deactivate its \
             RenderObjectElement"
        );
    }

    /// `RenderObjectElement.unmount`.
    fn unmount(self: Handle<Self>, app: &mut App)
    where
        Self: RenderObjectElementWidget,
    {
        let old_widget = self.as_element().widget(app).clone();
        ElementBase::unmount(self, app);
        let render_object = RenderObjectElement::render_object(self, app);
        debug_assert!(
            !render_object.attached(app),
            "A RenderObject was still attached when attempting to unmount its RenderObjectElement"
        );
        let typed = render_object
            .downcast(app)
            .expect("the render object this widget created");
        Self::widget_of(&old_widget).did_unmount_render_object(app, typed);
        render_object.dispose(app);
        self.render_object_element_data_mut(app).render_object = None;
    }

    /// `RenderObjectElement._updateParentData`.
    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        let render_object = RenderObjectElement::render_object(self, app);
        parent_data_element.apply_parent_data(app, render_object);
    }

    /// `RenderObjectElement.updateSlot`.
    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        let old_slot = self.as_element().slot(app);
        debug_assert!(old_slot != new_slot);
        ElementBase::update_slot(self, app, new_slot.clone());
        debug_assert!(self.as_element().slot(app) == new_slot);
        debug_assert!(
            self.render_object_element_data(app)
                .ancestor_render_object_element
                == self.find_ancestor_render_object_element(app)
        );
        if let Some(ancestor) = self
            .render_object_element_data(app)
            .ancestor_render_object_element
        {
            let render_object = RenderObjectElement::render_object(self, app);
            ancestor.move_render_object_child(app, render_object, old_slot, new_slot);
        }
    }

    /// `RenderObjectElement.attachRenderObject`.
    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        debug_assert!(
            self.render_object_element_data(app)
                .ancestor_render_object_element
                .is_none()
        );
        self.as_element().set_slot(app, new_slot.clone());
        let ancestor = self.find_ancestor_render_object_element(app);
        self.render_object_element_data_mut(app)
            .ancestor_render_object_element = ancestor;
        debug_assert!(
            ancestor.is_some(),
            "The render object for {:?} cannot find ancestor render object to attach to. Try \
             wrapping your widget in a View widget or any other widget that is backed by a \
             RenderTreeRootElement to serve as the root of the render tree.",
            self.as_element()
        );
        if let Some(ancestor) = ancestor {
            let render_object = RenderObjectElement::render_object(self, app);
            ancestor.insert_render_object_child(app, render_object, new_slot);
        }
        for parent_data_element in self.find_ancestor_parent_data_elements(app) {
            RenderObjectElement::update_parent_data(self, app, parent_data_element);
        }
    }

    /// `RenderObjectElement.detachRenderObject`.
    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        if let Some(ancestor) = self
            .render_object_element_data(app)
            .ancestor_render_object_element
        {
            let render_object = RenderObjectElement::render_object(self, app);
            let slot = self.as_element().slot(app);
            ancestor.remove_render_object_child(app, render_object, slot);
            self.render_object_element_data_mut(app)
                .ancestor_render_object_element = None;
        }
        self.as_element().set_slot(app, None);
    }
}

/// The widget type a [`RenderObjectElement`] is generic over, and how to read it off the
/// erased widget.
pub trait RenderObjectElementWidget {
    /// The widget class.
    type Widget: RenderObjectWidget;

    /// Dart's `widget as RenderObjectWidget`.
    fn widget_of(widget: &WidgetRef) -> &Self::Widget;
}

/// The `impl Element` overrides every render object element writes the same way.
macro_rules! render_object_element_overrides {
    () => {
        fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
            self.render_object_element_data(app).render_object
        }

        fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
            None
        }

        fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
            self.render_object_element_data(app).debug_doing_build
        }

        const IS_RENDER_OBJECT_ELEMENT: bool = true;

        #[inline(always)]
        fn mount(
            self: Handle<Self>,
            app: &mut App,
            parent: Option<AnyElement>,
            new_slot: Option<Slot>,
        ) {
            RenderObjectElement::mount(self, app, parent, new_slot);
        }

        #[inline(always)]
        fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
            RenderObjectElement::update(self, app, new_widget);
        }

        #[inline(always)]
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
            RenderObjectElement::update_slot(self, app, new_slot);
        }

        fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
            RenderObjectElement::attach_render_object(self, app, new_slot);
        }

        fn detach_render_object(self: Handle<Self>, app: &mut App) {
            RenderObjectElement::detach_render_object(self, app);
        }
    };
}

// ---------------------------------------------------------------------------------------------
// LeafRenderObjectElement

/// An [`Element`] that uses a [`LeafRenderObjectWidget`] as its configuration.
pub struct LeafRenderObjectElement<W: LeafRenderObjectWidget> {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: LeafRenderObjectWidget> LeafRenderObjectElement<W> {
    /// Creates an element that uses the given widget as its configuration.
    pub fn new(app: &mut App, widget: WidgetRef) -> Handle<Self> {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(LeafRenderObjectElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            marker: std::marker::PhantomData,
        })
        .into_tree_node(app)
    }
}

impl<W: LeafRenderObjectWidget> RenderObjectElementWidget for LeafRenderObjectElement<W> {
    type Widget = W;

    fn widget_of(widget: &WidgetRef) -> &W {
        downcast_widget::<W>(&**widget)
            .expect("a LeafRenderObjectElement holds its LeafRenderObjectWidget")
    }
}

impl<W: LeafRenderObjectWidget> RenderObjectElement for LeafRenderObjectElement<W> {
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

impl<W: LeafRenderObjectWidget> Element for LeafRenderObjectElement<W> {
    crate::element_accessors!();
    render_object_element_overrides!();

    fn forget_child(self: Handle<Self>, _app: &mut App, _child: AnyElement) {
        unreachable!("a leaf render object element has no child to forget");
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        unreachable!("a leaf render object element takes no child");
    }

    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        unreachable!("a leaf render object element takes no child");
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        unreachable!("a leaf render object element takes no child");
    }
}

// ---------------------------------------------------------------------------------------------
// SingleChildRenderObjectElement

/// The fields of Dart's `SingleChildRenderObjectElement`.
#[derive(Default)]
pub struct SingleChildRenderObjectElementData {
    child: Option<AnyElement>,
}

/// The bodies of Dart's `SingleChildRenderObjectElement`: an [`Element`] that uses a
/// [`SingleChildRenderObjectWidget`] as its configuration.
///
/// The child is optional.
///
/// This element subclass can be used for [`RenderObjectWidget`]s whose `RenderObject`s use the
/// `RenderObjectWithChildMixin` mixin. Such widgets are expected to inherit from
/// [`SingleChildRenderObjectWidget`].
///
/// Flutter's class is instantiable and also subclassed (`SingleChildScrollView`'s viewport
/// element), so the bodies live on this trait and [`SingleChildRenderObjectElement`] is the
/// plain leaf that implements it; a subclass calls the body it overrides by name where Dart
/// writes `super`.
pub trait SingleChildRenderObjectElementBase:
    RenderObjectElement + RenderObjectElementWidget
where
    Self::Widget: SingleChildRenderObjectWidget,
    <Self::Widget as RenderObjectWidget>::RenderObject: RenderObjectWithChildMixin,
{
    /// The child slot, held under the field `single_child`.
    fn single_child_data(self: Handle<Self>, app: &App) -> &SingleChildRenderObjectElementData;

    /// See [`single_child_data`](Self::single_child_data).
    fn single_child_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleChildRenderObjectElementData;

    /// The child element, if any.
    fn child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        self.single_child_data(app).child
    }

    /// Dart's `updateChild(_child, (widget as SingleChildRenderObjectWidget).child, null)`,
    /// which both `mount` and `update` run.
    #[inline(always)]
    fn update_child_from_widget(self: Handle<Self>, app: &mut App) {
        let child_widget = Self::widget_of(self.as_element().widget(app))
            .child()
            .cloned();
        let child = self.single_child_data(app).child;
        let child = self
            .as_element()
            .update_child(app, child, child_widget, None);
        self.single_child_data_mut(app).child = child;
    }

    /// `SingleChildRenderObjectElement.visitChildren`.
    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        if let Some(child) = self.single_child_data(app).child {
            visitor(child);
        }
    }

    /// `SingleChildRenderObjectElement.forgetChild`.
    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(self.single_child_data(app).child == Some(child));
        self.single_child_data_mut(app).child = None;
    }

    /// `SingleChildRenderObjectElement.mount`.
    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
        self.update_child_from_widget(app);
    }

    /// `SingleChildRenderObjectElement.update`.
    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        RenderObjectElement::update(self, app, new_widget);
        self.update_child_from_widget(app);
    }

    /// `SingleChildRenderObjectElement.insertRenderObjectChild`.
    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        debug_assert!(slot.is_none());
        let render_object = self.typed_render_object(app);
        render_object.set_child(app, Some(ErasedRenderObject::from_object(child)));
    }

    /// `SingleChildRenderObjectElement.moveRenderObjectChild`.
    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        unreachable!("a single child never moves slots");
    }

    /// `SingleChildRenderObjectElement.removeRenderObjectChild`.
    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        debug_assert!(slot.is_none());
        let render_object = self.typed_render_object(app);
        debug_assert!(render_object.child(app).map(|current| current.as_object()) == Some(child));
        render_object.set_child(app, None);
    }
}

/// An [`Element`] that uses a [`SingleChildRenderObjectWidget`] as its configuration: the leaf
/// that has no bodies of its own beyond [`SingleChildRenderObjectElementBase`]'s.
pub struct SingleChildRenderObjectElement<W: SingleChildRenderObjectWidget>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    element: ElementData,
    render_object_element: RenderObjectElementData,
    single_child: SingleChildRenderObjectElementData,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: SingleChildRenderObjectWidget> SingleChildRenderObjectElement<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    /// Creates an element that uses the given widget as its configuration.
    pub fn new(app: &mut App, widget: WidgetRef) -> Handle<Self> {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(SingleChildRenderObjectElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            single_child: SingleChildRenderObjectElementData::default(),
            marker: std::marker::PhantomData,
        })
        .into_tree_node(app)
    }
}

impl<W: SingleChildRenderObjectWidget> RenderObjectElementWidget
    for SingleChildRenderObjectElement<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    type Widget = W;

    fn widget_of(widget: &WidgetRef) -> &W {
        downcast_widget::<W>(&**widget)
            .expect("a SingleChildRenderObjectElement holds its SingleChildRenderObjectWidget")
    }
}

impl<W: SingleChildRenderObjectWidget> RenderObjectElement for SingleChildRenderObjectElement<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
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

impl<W: SingleChildRenderObjectWidget> SingleChildRenderObjectElementBase
    for SingleChildRenderObjectElement<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    fn single_child_data(self: Handle<Self>, app: &App) -> &SingleChildRenderObjectElementData {
        &app.get(self).single_child
    }

    fn single_child_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleChildRenderObjectElementData {
        &mut app.get_mut(self).single_child
    }
}

impl<W: SingleChildRenderObjectWidget> Element for SingleChildRenderObjectElement<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    crate::element_accessors!();

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build
    }

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        SingleChildRenderObjectElementBase::visit_children(self, app, visitor);
    }

    fn debug_visit_onstage_children(
        self: Handle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyElement),
    ) {
        let children: Vec<AnyElement> = self.child(app).into_iter().collect();
        Self::widget_of(self.as_element().widget(app))
            .debug_visit_onstage_children(app, &children, visitor);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        SingleChildRenderObjectElementBase::forget_child(self, app, child);
    }

    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::mount(self, app, parent, new_slot);
    }

    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        SingleChildRenderObjectElementBase::update(self, app, new_widget);
    }

    #[inline(always)]
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
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::insert_render_object_child(self, app, child, slot);
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::move_render_object_child(
            self, app, child, old_slot, new_slot,
        );
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::remove_render_object_child(self, app, child, slot);
    }
}

// ---------------------------------------------------------------------------------------------
// MultiChildRenderObjectElement

/// The fields of Dart's `MultiChildRenderObjectElement`.
#[derive(Default)]
pub struct MultiChildRenderObjectElementData {
    children: Vec<AnyElement>,
    // We keep a set of forgotten children to avoid O(n^2) work walking children repeatedly to
    // remove children.
    forgotten_children: HashSet<AnyElement>,
}

/// The bodies of Dart's `MultiChildRenderObjectElement`: an [`Element`] that uses a
/// [`MultiChildRenderObjectWidget`] as its configuration.
///
/// This element subclass can be used for [`RenderObjectWidget`]s whose `RenderObject`s use the
/// `ContainerRenderObjectMixin` mixin with a parent data type that implements
/// `ContainerParentDataMixin`. Such widgets are expected to inherit from
/// [`MultiChildRenderObjectWidget`].
///
/// Flutter's class is instantiable and also subclassed (`Viewport`'s element), so the bodies
/// live on this trait and [`MultiChildRenderObjectElement`] is the plain leaf that implements
/// it; a subclass calls the body it overrides by name where Dart writes `super`.
///
/// See also:
///
/// * [`IndexedSlot`], which is used as the [`Element`] slots for the children of a
///   [`MultiChildRenderObjectElement`].
/// * [`AnyElement::update_children`], which discusses why [`IndexedSlot`] is used for the slots
///   of the children.
pub trait MultiChildRenderObjectElementBase:
    RenderObjectElement + RenderObjectElementWidget
where
    Self::Widget: MultiChildRenderObjectWidget,
    <Self::Widget as RenderObjectWidget>::RenderObject: ContainerRenderObjectMixin,
{
    /// The child list, held under the field `multi_child`.
    fn multi_child_data(self: Handle<Self>, app: &App) -> &MultiChildRenderObjectElementData;

    /// See [`multi_child_data`](Self::multi_child_data).
    fn multi_child_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut MultiChildRenderObjectElementData;

    /// The current list of children of this element.
    ///
    /// This list is filtered to hide elements that have been forgotten (using
    /// [`Element::forget_child`]).
    fn children(self: Handle<Self>, app: &App) -> Vec<AnyElement> {
        let this = self.multi_child_data(app);
        this.children
            .iter()
            .copied()
            .filter(|child| !this.forgotten_children.contains(child))
            .collect()
    }

    /// Dart's `slot.value?.renderObject`: the child this child is inserted after, as the
    /// container's `ChildType`.
    fn after_render_object(
        app: &App,
        slot: Option<&Slot>,
    ) -> Option<
        <<Self::Widget as RenderObjectWidget>::RenderObject as ContainerRenderObjectMixin>::ChildType,
    >{
        let render_object = Self::indexed_slot(slot)
            .value?
            .render_object(app)
            .expect("the element before this slot has a render object");
        Some(ErasedRenderObject::from_object(render_object))
    }

    /// Dart's `IndexedSlot<Element?> slot` parameter type.
    fn indexed_slot(slot: Option<&Slot>) -> &IndexedSlot {
        match slot {
            Some(Slot::Indexed(slot)) => slot,
            _ => unreachable!("a multi-child element gives its children indexed slots"),
        }
    }

    /// `MultiChildRenderObjectElement.visitChildren`.
    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        let this = self.multi_child_data(app);
        for child in &this.children {
            if !this.forgotten_children.contains(child) {
                visitor(*child);
            }
        }
    }

    /// `MultiChildRenderObjectElement.forgetChild`.
    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(self.multi_child_data(app).children.contains(&child));
        debug_assert!(
            !self
                .multi_child_data(app)
                .forgotten_children
                .contains(&child)
        );
        self.multi_child_data_mut(app)
            .forgotten_children
            .insert(child);
    }

    /// `MultiChildRenderObjectElement.mount`.
    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
        let child_widgets = Self::widget_of(self.as_element().widget(app))
            .children()
            .to_vec();
        let mut children = Vec::with_capacity(child_widgets.len());
        let mut previous_child: Option<AnyElement> = None;
        for (index, child_widget) in child_widgets.into_iter().enumerate() {
            let new_child = self.as_element().inflate_widget(
                app,
                child_widget,
                Some(Slot::Indexed(IndexedSlot {
                    index,
                    value: previous_child,
                })),
            );
            children.push(new_child);
            previous_child = Some(new_child);
        }
        self.multi_child_data_mut(app).children = children;
    }

    /// `MultiChildRenderObjectElement.update`.
    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        RenderObjectElement::update(self, app, new_widget);
        let child_widgets = Self::widget_of(self.as_element().widget(app))
            .children()
            .to_vec();
        let old_children = self.multi_child_data(app).children.clone();
        let is_forgotten = |app: &App, child: AnyElement| {
            self.multi_child_data(app)
                .forgotten_children
                .contains(&child)
        };
        let children = self.as_element().update_children(
            app,
            &old_children,
            &child_widgets,
            Some(&is_forgotten),
            None,
        );
        let this = self.multi_child_data_mut(app);
        this.children = children;
        this.forgotten_children.clear();
    }

    /// `MultiChildRenderObjectElement.insertRenderObjectChild`.
    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let render_object = self.typed_render_object(app);
        let after = Self::after_render_object(app, slot.as_ref());
        render_object.insert(app, ErasedRenderObject::from_object(child), after);
    }

    /// `MultiChildRenderObjectElement.moveRenderObjectChild`.
    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        let render_object = self.typed_render_object(app);
        debug_assert!(child.parent(app) == Some(RenderObjectElement::render_object(self, app)));
        let after = Self::after_render_object(app, new_slot.as_ref());
        render_object.move_child(app, ErasedRenderObject::from_object(child), after);
    }

    /// `MultiChildRenderObjectElement.removeRenderObjectChild`.
    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        let render_object = self.typed_render_object(app);
        debug_assert!(child.parent(app) == Some(RenderObjectElement::render_object(self, app)));
        render_object.remove(app, ErasedRenderObject::from_object(child));
    }
}

/// An [`Element`] that uses a [`MultiChildRenderObjectWidget`] as its configuration: the leaf
/// that has no bodies of its own beyond [`MultiChildRenderObjectElementBase`]'s.
pub struct MultiChildRenderObjectElement<W: MultiChildRenderObjectWidget>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    element: ElementData,
    render_object_element: RenderObjectElementData,
    multi_child: MultiChildRenderObjectElementData,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: MultiChildRenderObjectWidget> MultiChildRenderObjectElement<W>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    /// Creates an element that uses the given widget as its configuration.
    pub fn new(app: &mut App, widget: WidgetRef) -> Handle<Self> {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(MultiChildRenderObjectElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            multi_child: MultiChildRenderObjectElementData::default(),
            marker: std::marker::PhantomData,
        })
        .into_tree_node(app)
    }
}

impl<W: MultiChildRenderObjectWidget> RenderObjectElementWidget for MultiChildRenderObjectElement<W>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    type Widget = W;

    fn widget_of(widget: &WidgetRef) -> &W {
        downcast_widget::<W>(&**widget)
            .expect("a MultiChildRenderObjectElement holds its MultiChildRenderObjectWidget")
    }
}

impl<W: MultiChildRenderObjectWidget> RenderObjectElement for MultiChildRenderObjectElement<W>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
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

impl<W: MultiChildRenderObjectWidget> MultiChildRenderObjectElementBase
    for MultiChildRenderObjectElement<W>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    fn multi_child_data(self: Handle<Self>, app: &App) -> &MultiChildRenderObjectElementData {
        &app.get(self).multi_child
    }

    fn multi_child_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut MultiChildRenderObjectElementData {
        &mut app.get_mut(self).multi_child
    }
}

impl<W: MultiChildRenderObjectWidget> Element for MultiChildRenderObjectElement<W>
where
    W::RenderObject: ContainerRenderObjectMixin,
{
    crate::element_accessors!();

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build
    }

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    #[inline(always)]
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
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        MultiChildRenderObjectElementBase::visit_children(self, app, visitor);
    }

    fn debug_visit_onstage_children(
        self: Handle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyElement),
    ) {
        let mut children = Vec::new();
        MultiChildRenderObjectElementBase::visit_children(self, app, &mut |child| {
            children.push(child);
        });
        Self::widget_of(self.as_element().widget(app))
            .debug_visit_onstage_children(app, &children, visitor);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        MultiChildRenderObjectElementBase::forget_child(self, app, child);
    }

    #[inline(always)]
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::mount(self, app, parent, new_slot);
    }

    #[inline(always)]
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        MultiChildRenderObjectElementBase::update(self, app, new_widget);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::insert_render_object_child(self, app, child, slot);
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::move_render_object_child(
            self, app, child, old_slot, new_slot,
        );
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::remove_render_object_child(self, app, child, slot);
    }
}

// ---------------------------------------------------------------------------------------------
// RenderTreeRootElement

/// The `attachRenderObject` / `detachRenderObject` / `updateSlot` bodies of a
/// `RenderTreeRootElement`: a render object element whose render object does not attach to
/// an ancestor, because it is the root of its own render tree (`RenderView`).
///
/// A concrete element writes its own `Element` impl and calls these where Dart's
/// `RenderTreeRootElement` would run.
pub trait RenderTreeRootElement: RenderObjectElement {
    /// `RenderTreeRootElement.attachRenderObject`.
    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        self.as_element().set_slot(app, new_slot);
        debug_assert!(self.debug_check_must_not_attach_render_object_to_ancestor(app));
    }

    /// `RenderTreeRootElement.detachRenderObject`.
    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        self.as_element().set_slot(app, None);
    }

    /// `RenderTreeRootElement.updateSlot`.
    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        ElementBase::update_slot(self, app, new_slot);
        debug_assert!(self.debug_check_must_not_attach_render_object_to_ancestor(app));
    }

    fn debug_check_must_not_attach_render_object_to_ancestor(
        self: Handle<Self>,
        app: &App,
    ) -> bool {
        if !cfg!(debug_assertions) {
            return true;
        }
        assert!(
            self.find_ancestor_render_object_element(app).is_none(),
            "The RenderObject for {:?} cannot maintain an independent render tree at its \
             current location. This RenderObject is the root of an independent render tree and \
             it cannot attach itself to an ancestor in an existing tree. The ancestor \
             RenderObject, however, expects that a child will be attached.",
            self.as_element()
        );
        true
    }
}

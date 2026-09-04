//! The concrete element classes: `ComponentElement` and its `StatelessElement` /
//! `StatefulElement` / `ProxyElement` (`InheritedElement`, `ParentDataElement`), and
//! `RenderObjectElement` with `LeafRenderObjectElement` / `SingleChildRenderObjectElement`
//! and the `RenderTreeRootElement` base.
//!
//! Flutter's base classes are traits holding the shared bodies; each concrete element is a
//! struct generic over its widget type, so the widget is reached without a cast.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

use reveal_foundation::{App, Handle, HandleId};
use reveal_rendering::{AnyRenderObject, RenderHandle, RenderObjectWithChildMixin};

use super::element::{
    AnyElement, Element, ElementBase, ElementData, ElementLifecycle, Slot,
    debug_check_owner_build_target_exists,
};
use super::state::{State, StateLifecycle};
use super::widget::{
    InheritedWidget, LeafRenderObjectWidget, ParentDataWidget, RenderObjectWidget,
    SingleChildRenderObjectWidget, StatefulWidget, StatelessWidget, WidgetRef, downcast_widget,
};

// ---------------------------------------------------------------------------------------------
// ComponentElement

/// The fields of Dart's `ComponentElement`.
#[derive(Default)]
pub struct ComponentElementData {
    child: Option<AnyElement>,
    debug_doing_build: bool,
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

    /// The child this element built, if any.
    fn child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        self.component_data(app).child
    }

    /// `ComponentElement.mount`: the base mount, then the first build.
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
    fn first_build(self: Handle<Self>, app: &mut App) {
        self.as_element().rebuild(app, false); // This eventually calls performRebuild.
    }

    /// Calls the [`build`](Self::build) method and then updates the widget tree.
    ///
    /// Called automatically during [`ComponentElement::mount`](Self::mount) to generate the
    /// first build, and by `rebuild` when the element needs updating.
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        if cfg!(debug_assertions) {
            self.component_data_mut(app).debug_doing_build = true;
        }
        let built = self.build(app);
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

// ---------------------------------------------------------------------------------------------
// StatelessElement

/// An [`Element`] that uses a [`StatelessWidget`] as its configuration.
pub struct StatelessElement<W: StatelessWidget> {
    element: ElementData,
    component: ComponentElementData,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: StatelessWidget> StatelessElement<W> {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(StatelessElement::<W> {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            marker: std::marker::PhantomData,
        })
        .as_element()
    }
}

impl<W: StatelessWidget> ComponentElement for StatelessElement<W> {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.as_element().widget(app).clone();
        let stateless =
            &downcast_widget::<W>(&*widget).expect("a StatelessElement holds its StatelessWidget");
        stateless.build(app, self.as_element())
    }
}

impl<W: StatelessWidget> Element for StatelessElement<W> {
    crate::element_accessors!();
    component_element_overrides!();

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ElementBase::update(self, app, new_widget);
        self.as_element().rebuild(app, true);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }
}

// ---------------------------------------------------------------------------------------------
// StatefulElement

/// An [`Element`] that uses a [`StatefulWidget`] as its configuration.
pub struct StatefulElement<W: StatefulWidget> {
    element: ElementData,
    component: ComponentElementData,
    state: Option<Handle<W::State>>,
    did_change_dependencies: bool,
}

impl<W: StatefulWidget> StatefulElement<W> {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        let state = {
            let stateful = &downcast_widget::<W>(&*widget)
                .expect("a StatefulElement holds its StatefulWidget");
            stateful.create_state()
        };
        let state = app.create(state);
        let this = app.create(StatefulElement::<W> {
            element: ElementData::new(widget.clone()),
            component: ComponentElementData::default(),
            state: Some(state),
            did_change_dependencies: false,
        });
        debug_assert!(state.state_data(app).element().is_none());
        state.state_data_mut(app).set_element(Some(this));
        state.state_data_mut(app).set_widget(widget);
        debug_assert!(state.state_data(app).debug_lifecycle_state() == StateLifecycle::Created);
        this.as_element()
    }

    /// The [`State`] instance associated with this location in the tree.
    ///
    /// There is a one-to-one relationship between [`State`] objects and the
    /// [`StatefulElement`] objects that hold them. The [`State`] objects are created by
    /// [`StatefulElement`] in `mount`.
    pub fn state(self: Handle<Self>, app: &App) -> Handle<W::State> {
        app.get(self).state.expect("the element is unmounted")
    }

    /// The widget, typed.
    pub fn widget(self: Handle<Self>, app: &App) -> &W {
        downcast_widget::<W>(&**self.as_element().widget(app))
            .expect("a StatefulElement holds its StatefulWidget")
    }
}

impl<W: StatefulWidget> ComponentElement for StatefulElement<W> {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let state = self.state(app);
        state.build(app, self.as_element())
    }

    fn first_build(self: Handle<Self>, app: &mut App) {
        let state = self.state(app);
        debug_assert!(state.state_data(app).debug_lifecycle_state() == StateLifecycle::Created);
        state.init_state(app);
        if cfg!(debug_assertions) {
            state
                .state_data_mut(app)
                .set_debug_lifecycle_state(StateLifecycle::Initialized);
        }
        state.did_change_dependencies(app);
        if cfg!(debug_assertions) {
            state
                .state_data_mut(app)
                .set_debug_lifecycle_state(StateLifecycle::Ready);
        }
        self.as_element().rebuild(app, false);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        if app.get(self).did_change_dependencies {
            self.state(app).did_change_dependencies(app);
            app.get_mut(self).did_change_dependencies = false;
        }
        if cfg!(debug_assertions) {
            self.component_data_mut(app).debug_doing_build = true;
        }
        let built = self.build(app);
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

impl<W: StatefulWidget> Element for StatefulElement<W> {
    crate::element_accessors!();
    component_element_overrides!();

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        ElementBase::update(self, app, new_widget.clone());
        let state = self.state(app);
        state.state_data_mut(app).set_widget(new_widget);
        let old = &downcast_widget::<W>(&*old_widget)
            .expect("a StatefulElement holds its StatefulWidget");
        state.did_update_widget(app, old);
        self.as_element().rebuild(app, true);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        ElementBase::activate(self, app);
        self.state(app).activate(app);
        // Since the State could have observed the deactivate() and thus disposed of
        // resources allocated in the build method, we have to rebuild the widget
        // so that its State can reallocate its resources.
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Active); // otherwise markNeedsBuild is a no-op
        self.as_element().mark_needs_build(app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        self.state(app).deactivate(app);
        ElementBase::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        ElementBase::unmount(self, app);
        let state = self.state(app);
        state.dispose(app);
        if cfg!(debug_assertions) {
            state
                .state_data_mut(app)
                .set_debug_lifecycle_state(StateLifecycle::Defunct);
        }
        state.state_data_mut(app).set_element(None);
        app.get_mut(self).state = None;
        app.destroy(state);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        ElementBase::did_change_dependencies(self, app);
        app.get_mut(self).did_change_dependencies = true;
    }

    fn state_handle_id(self: Handle<Self>, app: &App) -> Option<(TypeId, HandleId)> {
        app.get(self)
            .state
            .map(|state| (TypeId::of::<W::State>(), state.id()))
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
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        debug_assert!(!Rc::ptr_eq(&old_widget, &new_widget));
        ElementBase::update(self, app, new_widget);
        self.updated(app, old_widget);
        self.as_element().rebuild(app, true);
    }
}

// ---------------------------------------------------------------------------------------------
// InheritedElement

/// An [`Element`] that uses an [`InheritedWidget`] as its configuration.
pub struct InheritedElement<W: InheritedWidget> {
    element: ElementData,
    component: ComponentElementData,
    dependents: HashMap<AnyElement, Option<Rc<dyn Any>>>,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: InheritedWidget> InheritedElement<W> {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(InheritedElement::<W> {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            dependents: HashMap::new(),
            marker: std::marker::PhantomData,
        })
        .as_element()
    }

    /// The widget, typed.
    pub fn widget(self: Handle<Self>, app: &App) -> &W {
        downcast_widget::<W>(&**self.as_element().widget(app))
            .expect("an InheritedElement holds its InheritedWidget")
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

impl<W: InheritedWidget> ComponentElement for InheritedElement<W> {
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

impl<W: InheritedWidget> ProxyElement for InheritedElement<W> {
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
        self.widget(app).child().clone()
    }

    fn updated(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        let should_notify = {
            let old =
                &downcast_widget::<W>(&*old_widget).expect("the old widget has the same type");
            self.widget(app).update_should_notify(old)
        };
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

impl<W: InheritedWidget> Element for InheritedElement<W> {
    crate::element_accessors!();
    component_element_overrides!();

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ProxyElement::update(self, app, new_widget);
    }

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
        widgets.insert(TypeId::of::<W>(), self.as_element());
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
pub struct ParentDataElement<W: ParentDataWidget> {
    element: ElementData,
    component: ComponentElementData,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: ParentDataWidget> ParentDataElement<W> {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(ParentDataElement::<W> {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            marker: std::marker::PhantomData,
        })
        .as_element()
    }

    /// The widget, typed.
    pub fn widget(self: Handle<Self>, app: &App) -> &W {
        downcast_widget::<W>(&**self.as_element().widget(app))
            .expect("a ParentDataElement holds its ParentDataWidget")
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
    pub fn apply_widget_out_of_turn(self: Handle<Self>, app: &mut App, new_widget: &W) {
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

impl<W: ParentDataWidget> ComponentElement for ParentDataElement<W> {
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

impl<W: ParentDataWidget> ProxyElement for ParentDataElement<W> {
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
        self.widget(app).child().clone()
    }

    fn notify_clients(self: Handle<Self>, app: &mut App, _old_widget: WidgetRef) {
        self.apply_parent_data_to_descendants(app);
    }
}

impl<W: ParentDataWidget> Element for ParentDataElement<W> {
    crate::element_accessors!();
    component_element_overrides!();

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ProxyElement::update(self, app, new_widget);
    }

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
        let valid = render_object.parent_data_is::<W::ParentData>(app);
        debug_assert!(
            valid,
            "Incorrect use of ParentDataWidget. The ParentDataWidget {:?} wants to apply \
             ParentData of type {} to a RenderObject which has been set up to accept ParentData \
             of an incompatible type. Usually, this means that the widget has the wrong \
             ancestor RenderObjectWidget.",
            self.widget(app),
            std::any::type_name::<W::ParentData>()
        );
        if valid {
            let widget = self.as_element().widget(app).clone();
            let typed = &downcast_widget::<W>(&*widget)
                .expect("a ParentDataElement holds its ParentDataWidget");
            typed.apply_parent_data(app, render_object);
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
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef)
    where
        Self: RenderObjectElementWidget,
    {
        ElementBase::update(self, app, new_widget);
        RenderObjectElement::perform_rebuild(self, app); // calls widget.updateRenderObject()
    }

    /// `RenderObjectElement._performRebuild`: push the widget's configuration.
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

        fn mount(
            self: Handle<Self>,
            app: &mut App,
            parent: Option<AnyElement>,
            new_slot: Option<Slot>,
        ) {
            RenderObjectElement::mount(self, app, parent, new_slot);
        }

        fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
            RenderObjectElement::update(self, app, new_widget);
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
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(LeafRenderObjectElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            marker: std::marker::PhantomData,
        })
        .as_element()
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

/// An [`Element`] that uses a [`SingleChildRenderObjectWidget`] as its configuration.
///
/// The child is optional.
///
/// This element subclass can be used for `RenderObjectWidget`s whose `RenderObject`s use the
/// `RenderObjectWithChildMixin` mixin. Such widgets are expected to inherit from
/// [`SingleChildRenderObjectWidget`].
pub struct SingleChildRenderObjectElement<W: SingleChildRenderObjectWidget>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    element: ElementData,
    render_object_element: RenderObjectElementData,
    child: Option<AnyElement>,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: SingleChildRenderObjectWidget> SingleChildRenderObjectElement<W>
where
    W::RenderObject: RenderObjectWithChildMixin,
{
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        app.create(SingleChildRenderObjectElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            child: None,
            marker: std::marker::PhantomData,
        })
        .as_element()
    }

    /// The child element, if any.
    pub fn child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        app.get(self).child
    }

    fn update_child_from_widget(self: Handle<Self>, app: &mut App) {
        let child_widget = Self::widget_of(self.as_element().widget(app))
            .child()
            .cloned();
        let child = app.get(self).child;
        let child = self
            .as_element()
            .update_child(app, child, child_widget, None);
        app.get_mut(self).child = child;
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
        debug_assert!(slot.is_none());
        let render_object = self.typed_render_object(app);
        let child = child
            .as_box()
            .expect("a single-child render object holds a box");
        render_object.set_child(app, Some(child));
    }

    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        unreachable!("a single child never moves slots");
    }

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

//! Flutter counterpart: `widgets/selection_container.dart`.
//!
//! A container that handles [`SelectionEvent`]s for the selectables in a subtree. It hosts its
//! delegate through [`SelectionRegistrarScope`], and registers itself with the registrar above it
//! as one selectable.

use waft_embedder::{Matrix4, Rect, Size};
use waft_foundation::{App, Handle, HandleId, ListenableObject, Listener, Ref};
use waft_rendering::{
    AnyRenderObject, AnySelectable, AnySelectionRegistrar, LayerLink, Selectable, SelectedContent,
    SelectedContentRange, SelectionEvent, SelectionGeometry, SelectionHandler, SelectionRegistrant,
    SelectionRegistrantData, SelectionRegistrar, SelectionRegistrarRef, SelectionResult,
    SelectionStatus,
};

use crate::framework::IntoWidget;
use crate::framework::{
    BuildContext, InheritedWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};

/// A container that handles selection events for the selectables in its subtree.
///
/// Its state is a single selectable, and registers itself with the [`registrar`](Self::registrar)
/// when one is given and with the registrar from the context otherwise. The selectables below it
/// are collected by [`delegate`](Self::delegate), which the state hosts as the subtree's
/// [`SelectionRegistrar`].
pub struct SelectionContainer {
    pub key: Option<KeyRef>,

    /// The registrar this container is registered to; the context's when `None`.
    pub registrar: Option<SelectionRegistrarRef>,

    /// The delegate for the events sent to this container; `None` disables selection.
    pub delegate: Option<SelectionContainerDelegateRef>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

/// A field that keeps a selection container delegate: the widget's.
pub type SelectionContainerDelegateRef = waft_foundation::Retained<AnySelectionContainerDelegate>;

impl SelectionContainer {
    /// Creates a selection container for `child` that collects its selectables through
    /// `delegate`.
    pub fn new<K>(delegate: SelectionContainerDelegateRef, child: impl IntoWidget<K>) -> Self {
        SelectionContainer {
            key: None,
            registrar: None,
            delegate: Some(delegate),
            child: child.into_widget(),
        }
    }

    /// Creates a selection container that disables selection for its subtree.
    pub fn disabled<K>(child: impl IntoWidget<K>) -> Self {
        SelectionContainer {
            key: None,
            registrar: None,
            delegate: None,
            child: child.into_widget(),
        }
    }

    /// Dart `SelectionContainer(key:)`.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// Dart `SelectionContainer(registrar:)`.
    pub fn registrar(mut self, registrar: SelectionRegistrarRef) -> Self {
        self.registrar = Some(registrar);
        self
    }

    /// The immediate ancestor [`SelectionRegistrar`] of `context`, `None` when there is no
    /// selection container above it or the immediate one is disabled.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<SelectionRegistrarRef> {
        context
            .depend_on_inherited_widget_of_exact_type::<SelectionRegistrarScope>(app)
            .and_then(|scope| scope.registrar.clone())
    }

    fn is_disabled(&self) -> bool {
        self.delegate.is_none()
    }

    fn delegate_handle(&self) -> Option<AnySelectionContainerDelegate> {
        self.delegate
            .as_ref()
            .map(SelectionContainerDelegateRef::handle)
    }
}

impl std::fmt::Debug for SelectionContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionContainer")
            .field("delegate", &self.delegate)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectionContainer {
    type State = SelectionContainerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SelectionContainerState {
        SelectionContainerState {
            state: StateData::new(),
            selection_registrant: SelectionRegistrantData::new(),
            listeners: Vec::new(),
        }
    }
}

/// The state of a [`SelectionContainer`]: Dart's `_SelectionContainerState`.
///
/// It is a [`Selectable`] whose listeners are the delegate's: adding one adds it to the delegate
/// and remembers it, so a new delegate receives them all.
pub struct SelectionContainerState {
    state: StateData<SelectionContainer>,
    selection_registrant: SelectionRegistrantData,
    listeners: Vec<Listener>,
}

const DISABLED_GEOMETRY: SelectionGeometry = SelectionGeometry {
    start_selection_point: None,
    end_selection_point: None,
    status: SelectionStatus::None,
    selection_rects: Vec::new(),
    has_content: true,
};

impl SelectionContainerState {
    /// The enabled widget's delegate; Dart's `widget.delegate!`.
    fn delegate(self: Handle<Self>, app: &App) -> AnySelectionContainerDelegate {
        self.widget(app)
            .delegate_handle()
            .expect("an enabled selection container has a delegate")
    }
}

impl State for SelectionContainerState {
    type Widget = SelectionContainer;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        if !widget.is_disabled() {
            let registrar = widget.registrar.clone();
            self.delegate(app)
                .set_selection_container_context(app, Some(self.context(app)));
            if let Some(registrar) = registrar {
                self.set_registrar(app, Some(registrar));
            }
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SelectionContainer) {
        let widget = self.widget(app);
        let new_delegate = widget.delegate_handle();
        let disabled = widget.is_disabled();
        let registrar = widget.registrar.clone();
        let old_delegate = old_widget.delegate_handle();
        if old_delegate != new_delegate {
            let listeners = app.get(self).listeners.clone();
            if let Some(old_delegate) = old_delegate {
                old_delegate.set_selection_container_context(app, None);
                for listener in &listeners {
                    old_delegate.remove_listener(app, listener);
                }
            }
            if let Some(new_delegate) = new_delegate {
                new_delegate.set_selection_container_context(app, Some(self.context(app)));
                for listener in &listeners {
                    new_delegate.add_listener(app, listener.clone());
                }
            }
            let old_value = old_delegate.map(|delegate| delegate.value(app));
            let new_value = new_delegate.map(|delegate| delegate.value(app));
            if old_value != new_value {
                for listener in &listeners {
                    listener.call(app);
                }
            }
        }
        if disabled {
            self.set_registrar(app, None);
        } else if let Some(registrar) = registrar {
            self.set_registrar(app, Some(registrar));
        }
        debug_assert!(!disabled || self.registrar(app).is_none());
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let disabled = widget.is_disabled();
        if widget.registrar.is_none() && !disabled {
            let registrar = SelectionContainer::maybe_of(app, self.context(app));
            self.set_registrar(app, registrar);
        }
        debug_assert!(!disabled || self.registrar(app).is_none());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).is_disabled() {
            let delegate = self.delegate(app);
            delegate.set_selection_container_context(app, None);
            for listener in app.get(self).listeners.clone() {
                delegate.remove_listener(app, &listener);
            }
        }
        self.dispose_selection_registrant(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        match &widget.delegate {
            None => SelectionRegistrarScope::disabled(widget.child.clone()).into_widget(),
            Some(delegate) => {
                SelectionRegistrarScope::new(delegate.erase(), widget.child.clone()).into_widget()
            }
        }
    }
}

impl ListenableObject for SelectionContainerState {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        debug_assert!(!self.widget(app).is_disabled());
        self.delegate(app).add_listener(app, listener.clone());
        let listeners = &mut app.get_mut(self).listeners;
        if !listeners.contains(&listener) {
            listeners.push(listener);
        }
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        if let Some(delegate) = self.widget(app).delegate_handle() {
            delegate.remove_listener(app, listener);
        }
        app.get_mut(self)
            .listeners
            .retain(|registered| registered != listener);
    }
}

impl SelectionHandler for SelectionContainerState {
    fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
        if self.widget(app).is_disabled() {
            return DISABLED_GEOMETRY;
        }
        self.delegate(app).value(app)
    }

    fn push_handle_layers(
        self: Handle<Self>,
        app: &mut App,
        start_handle: Option<Ref<LayerLink>>,
        end_handle: Option<Ref<LayerLink>>,
    ) {
        self.delegate(app)
            .push_handle_layers(app, start_handle, end_handle);
    }

    fn selected_content(self: Handle<Self>, app: &App) -> Option<SelectedContent> {
        self.delegate(app).selected_content(app)
    }

    fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
        self.delegate(app).selection(app)
    }

    fn dispatch_selection_event(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        self.delegate(app).dispatch_selection_event(app, event)
    }

    fn content_length(self: Handle<Self>, app: &App) -> usize {
        self.delegate(app).content_length(app)
    }
}

impl Selectable for SelectionContainerState {
    fn get_transform_to(
        self: Handle<Self>,
        app: &App,
        ancestor: Option<AnyRenderObject>,
    ) -> Matrix4 {
        debug_assert!(!self.widget(app).is_disabled());
        container_render_object(app, self.context(app)).get_transform_to(app, ancestor)
    }

    fn size(self: Handle<Self>, app: &mut App) -> Size {
        container_render_box(app, self.context(app)).size(app)
    }

    fn bounding_boxes(self: Handle<Self>, app: &mut App) -> Vec<Rect> {
        vec![
            container_render_box(app, self.context(app))
                .as_object()
                .paint_bounds(app),
        ]
    }

    fn dispose_selectable(self: Handle<Self>, app: &mut App) {
        self.dispose_selection_registrant(app);
    }
}

impl SelectionRegistrant for SelectionContainerState {
    fn selection_registrant_data(self: Handle<Self>, app: &App) -> &SelectionRegistrantData {
        &app.get(self).selection_registrant
    }

    fn selection_registrant_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SelectionRegistrantData {
        &mut app.get_mut(self).selection_registrant
    }
}

/// `context.findRenderObject()!`.
fn container_render_object(app: &App, context: BuildContext) -> AnyRenderObject {
    context
        .find_render_object(app)
        .expect("a selection container's context has a render object once it is built")
}

/// `context.findRenderObject()! as RenderBox`.
fn container_render_box(app: &App, context: BuildContext) -> waft_rendering::AnyRenderBox {
    container_render_object(app, context)
        .as_box()
        .expect("a selection container's render object is a box")
}

/// An inherited widget that hosts a [`SelectionRegistrar`] for its subtree.
pub struct SelectionRegistrarScope {
    pub key: Option<KeyRef>,

    /// The registrar hosted by this scope; `None` disables selection for the subtree.
    pub registrar: Option<SelectionRegistrarRef>,

    pub child: WidgetRef,
}

impl SelectionRegistrarScope {
    /// Creates a scope that hosts `registrar`.
    pub fn new<K>(registrar: SelectionRegistrarRef, child: impl IntoWidget<K>) -> Self {
        SelectionRegistrarScope {
            key: None,
            registrar: Some(registrar),
            child: child.into_widget(),
        }
    }

    fn disabled(child: WidgetRef) -> Self {
        SelectionRegistrarScope {
            key: None,
            registrar: None,
            child,
        }
    }

    /// Dart `SelectionRegistrarScope(key:)`.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl std::fmt::Debug for SelectionRegistrarScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionRegistrarScope")
            .field("registrar", &self.registrar)
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for SelectionRegistrarScope {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &SelectionRegistrarScope) -> bool {
        old_widget.registrar != self.registrar
    }
}

/// The fields of Dart's `SelectionContainerDelegate`.
#[derive(Debug, Default)]
pub struct SelectionContainerDelegateData {
    selection_container_context: Option<BuildContext>,
}

impl SelectionContainerDelegateData {
    /// Creates delegate fields with no container context.
    pub fn new() -> Self {
        Self::default()
    }
}

/// A delegate to handle selection events for a [`SelectionContainer`].
///
/// The delegate also receives the selectables in the subtree through [`SelectionRegistrar`].
pub trait SelectionContainerDelegate: SelectionHandler + SelectionRegistrar {
    /// Erases this delegate for a [`SelectionContainer`], as Dart's implicit upcast.
    fn as_selection_container_delegate(self: Handle<Self>) -> AnySelectionContainerDelegate {
        AnySelectionContainerDelegate {
            id: self.id(),
            vtable: const { &SelectionContainerDelegateVTable::of::<Self>() },
        }
    }

    /// The fields of this class.
    fn selection_container_delegate_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SelectionContainerDelegateData;

    /// The fields of this class, mutably.
    fn selection_container_delegate_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SelectionContainerDelegateData;

    /// Gets the paint transform from the [`Selectable`] child to the [`SelectionContainer`] of
    /// this delegate; the container must have been laid out.
    fn get_transform_from(self: Handle<Self>, app: &App, child: AnySelectable) -> Matrix4 {
        let container = container_render_object(app, self.selection_container_context(app));
        child.get_transform_to(app, Some(container))
    }

    /// Gets the paint transform from the [`SelectionContainer`] of this delegate to `ancestor`;
    /// the container must have been laid out.
    fn get_transform_to(
        self: Handle<Self>,
        app: &App,
        ancestor: Option<AnyRenderObject>,
    ) -> Matrix4 {
        container_render_object(app, self.selection_container_context(app))
            .get_transform_to(app, ancestor)
    }

    /// Whether the [`SelectionContainer`] has undergone layout and has a size.
    fn has_size(self: Handle<Self>, app: &App) -> bool {
        container_render_box(app, self.selection_container_context(app)).has_size(app)
    }

    /// Gets the size of the [`SelectionContainer`] of this delegate; it must have been laid out.
    fn container_size(self: Handle<Self>, app: &App) -> Size {
        debug_assert!(
            self.has_size(app),
            "containerSize cannot be called before SelectionContainer is laid out."
        );
        container_render_box(app, self.selection_container_context(app)).size(app)
    }

    /// Dart's `_selectionContainerContext!`: the context of the container this delegate serves.
    fn selection_container_context(self: Handle<Self>, app: &App) -> BuildContext {
        self.selection_container_delegate_data(app)
            .selection_container_context
            .expect("the delegate's SelectionContainer has been built")
    }
}

fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

/// Erased [`SelectionContainerDelegate`], the type a [`SelectionContainer`] holds.
#[derive(Clone, Copy)]
pub struct AnySelectionContainerDelegate {
    id: HandleId,
    vtable: &'static SelectionContainerDelegateVTable,
}

struct SelectionContainerDelegateVTable {
    value: fn(&App, HandleId) -> SelectionGeometry,
    push_handle_layers: waft_rendering::PushHandleLayers,
    selected_content: fn(&App, HandleId) -> Option<SelectedContent>,
    selection: fn(&App, HandleId) -> Option<SelectedContentRange>,
    dispatch_selection_event: fn(&mut App, HandleId, &SelectionEvent) -> SelectionResult,
    content_length: fn(&App, HandleId) -> usize,
    add: fn(&mut App, HandleId, AnySelectable),
    remove: fn(&mut App, HandleId, AnySelectable),
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    set_selection_container_context: fn(&mut App, HandleId, Option<BuildContext>),
    get_transform_from: fn(&App, HandleId, AnySelectable) -> Matrix4,
    get_transform_to: fn(&App, HandleId, Option<AnyRenderObject>) -> Matrix4,
    has_size: fn(&App, HandleId) -> bool,
    container_size: fn(&App, HandleId) -> Size,
    as_selection_registrar: fn(HandleId) -> AnySelectionRegistrar,
}

impl SelectionContainerDelegateVTable {
    const fn of<A: SelectionContainerDelegate>() -> SelectionContainerDelegateVTable {
        SelectionContainerDelegateVTable {
            value: |app, id| A::value(resolve(id), app),
            push_handle_layers: |app, id, start, end| {
                A::push_handle_layers(resolve(id), app, start, end)
            },
            selected_content: |app, id| A::selected_content(resolve(id), app),
            selection: |app, id| A::selection(resolve(id), app),
            dispatch_selection_event: |app, id, event| {
                A::dispatch_selection_event(resolve(id), app, event)
            },
            content_length: |app, id| A::content_length(resolve(id), app),
            add: |app, id, selectable| A::add(resolve(id), app, selectable),
            remove: |app, id, selectable| A::remove(resolve(id), app, selectable),
            add_listener: |app, id, listener| {
                ListenableObject::add_listener(resolve::<A>(id), app, listener)
            },
            remove_listener: |app, id, listener| {
                ListenableObject::remove_listener(resolve::<A>(id), app, listener)
            },
            set_selection_container_context: |app, id, context| {
                A::selection_container_delegate_data_mut(resolve(id), app)
                    .selection_container_context = context;
            },
            get_transform_from: |app, id, child| A::get_transform_from(resolve(id), app, child),
            get_transform_to: |app, id, ancestor| A::get_transform_to(resolve(id), app, ancestor),
            has_size: |app, id| A::has_size(resolve(id), app),
            container_size: |app, id| A::container_size(resolve(id), app),
            as_selection_registrar: |id| {
                SelectionRegistrar::as_selection_registrar(resolve::<A>(id))
            },
        }
    }
}

impl AnySelectionContainerDelegate {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    pub fn value(self, app: &App) -> SelectionGeometry {
        (self.vtable.value)(app, self.id)
    }

    pub fn push_handle_layers(
        self,
        app: &mut App,
        start_handle: Option<Ref<LayerLink>>,
        end_handle: Option<Ref<LayerLink>>,
    ) {
        (self.vtable.push_handle_layers)(app, self.id, start_handle, end_handle)
    }

    pub fn selected_content(self, app: &App) -> Option<SelectedContent> {
        (self.vtable.selected_content)(app, self.id)
    }

    pub fn selection(self, app: &App) -> Option<SelectedContentRange> {
        (self.vtable.selection)(app, self.id)
    }

    pub fn dispatch_selection_event(
        self,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        (self.vtable.dispatch_selection_event)(app, self.id, event)
    }

    pub fn content_length(self, app: &App) -> usize {
        (self.vtable.content_length)(app, self.id)
    }

    pub fn add(self, app: &mut App, selectable: AnySelectable) {
        (self.vtable.add)(app, self.id, selectable)
    }

    pub fn remove(self, app: &mut App, selectable: AnySelectable) {
        (self.vtable.remove)(app, self.id, selectable)
    }

    pub fn add_listener(self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener)
    }

    pub fn remove_listener(self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener)
    }

    pub fn set_selection_container_context(self, app: &mut App, context: Option<BuildContext>) {
        (self.vtable.set_selection_container_context)(app, self.id, context)
    }

    pub fn get_transform_from(self, app: &App, child: AnySelectable) -> Matrix4 {
        (self.vtable.get_transform_from)(app, self.id, child)
    }

    pub fn get_transform_to(self, app: &App, ancestor: Option<AnyRenderObject>) -> Matrix4 {
        (self.vtable.get_transform_to)(app, self.id, ancestor)
    }

    pub fn has_size(self, app: &App) -> bool {
        (self.vtable.has_size)(app, self.id)
    }

    pub fn container_size(self, app: &App) -> Size {
        (self.vtable.container_size)(app, self.id)
    }

    pub fn as_selection_registrar(self) -> AnySelectionRegistrar {
        (self.vtable.as_selection_registrar)(self.id)
    }
}

impl std::fmt::Debug for AnySelectionContainerDelegate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnySelectionContainerDelegate")
            .field("id", &self.id)
            .finish()
    }
}

waft_foundation::erased_handle!(
    AnySelectionContainerDelegate: SelectionContainerDelegate => SelectionContainerDelegate::as_selection_container_delegate
);

/// The delegate as the registrar its container hosts: Dart's implicit upcast of
/// `SelectionContainerDelegate implements SelectionRegistrar`.
impl From<AnySelectionContainerDelegate> for AnySelectionRegistrar {
    fn from(delegate: AnySelectionContainerDelegate) -> AnySelectionRegistrar {
        delegate.as_selection_registrar()
    }
}

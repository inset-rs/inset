//! `Element`: an instantiation of a `Widget` at a particular location in the tree.
//!
//! An element is one struct in the arena, reached through `Handle<Self>`; Dart's base-class
//! fields are the [`ElementData`] bag the struct holds under the field `element`. A reference
//! to "some element" is the type-erased handle [`AnyElement`], which is also the [`BuildContext`]
//! (Dart's `Element implements BuildContext`).
//!
//! Dart's base bodies are the free functions `base_*`; an override calls the one it would
//! call as `super`.

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use reveal_embedder::Size;
use reveal_foundation::{App, Handle, HandleId};
use reveal_rendering::{AnyRenderObject, RenderHandle, RenderObject};

use super::build_owner::{BuildOwner, BuildScope};
use super::state::State;
use super::widget::{
    InheritedWidget, Widget, WidgetKind, WidgetRef, can_update, downcast_widget, global_key_id,
    same_widget,
};
use crate::widgets::scroll_notification::ScrollNotification;

/// A handle to the location of a widget in the widget tree.
///
/// This is Dart's `BuildContext` interface, which `Element` implements: here the type-erased
/// element handle plays both parts.
pub type BuildContext = AnyElement;

/// Dart's `_ElementLifecycle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementLifecycle {
    Initial,
    Active,
    Inactive,
    Failed,
    Defunct,
}

/// A value for `Element.slot` used for children of `MultiChildRenderObjectElement`s.
///
/// A slot for a `MultiChildRenderObjectElement` consists of an [`index`](Self::index)
/// identifying where the child occurring at this slot is located in the
/// `MultiChildRenderObjectElement`'s child list and an arbitrary [`value`](Self::value) that
/// can further define where the child occurring at this slot fits in its parent's child list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedSlot {
    /// Information to define where the child occurring at this slot fits in its parent's
    /// child list.
    pub value: Option<AnyElement>,
    /// The index of this slot in the parent's child list.
    pub index: usize,
}

/// Dart's `Object? slot` on an element: how the parent tells its children apart.
#[derive(Clone, Debug)]
pub enum Slot {
    /// A child of a multi-child render object element.
    Indexed(IndexedSlot),
    /// The index of a child of a lazily built sliver (`SliverMultiBoxAdaptorElement`).
    Index(i32),
    /// A parent-defined value, compared by identity as an arbitrary Dart object would be.
    Custom(Rc<dyn Any>),
}

impl PartialEq for Slot {
    fn eq(&self, other: &Slot) -> bool {
        match (self, other) {
            (Slot::Indexed(a), Slot::Indexed(b)) => a == b,
            (Slot::Index(a), Slot::Index(b)) => a == b,
            (Slot::Custom(a), Slot::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// The live "is this child forgotten?" check [`AnyElement::update_children`] takes (Dart's
/// `Set<Element> forgottenChildren`).
pub type ForgottenChildren<'a> = &'a dyn Fn(&App, AnyElement) -> bool;

/// A type a `NotificationListener<T>` can listen for: a concrete [`Notification`], or a
/// family of them named by a trait object (`dyn ScrollNotification`).
///
/// Dart's `notification is T`; a family answers it through a vtable slot on
/// [`Notification`], a concrete type through [`Notification::as_any`].
pub trait NotificationTarget: 'static {
    /// The notification as a `T`, when it is one.
    fn cast(notification: &dyn Notification) -> Option<&Self>;
}

impl<T: Notification> NotificationTarget for T {
    fn cast(notification: &dyn Notification) -> Option<&T> {
        notification.as_any().downcast_ref::<T>()
    }
}

/// To send a notification, call [`dispatch`](Self::dispatch) on the notification you wish to
/// send. The notification will be delivered to any `NotificationListener` widgets with the
/// appropriate type parameters that are ancestors of the given [`BuildContext`].
/// A notification that can bubble up the widget tree.
///
/// You can determine the type of a notification using the `is` operator to check the
/// `runtimeType` of the notification; here [`as_any`](Self::as_any) and a downcast.
///
/// To listen for notifications in a subtree, use a `NotificationListener`.
///
pub trait Notification: Any + Debug {
    /// The notification as `Any`, for Dart's `notification is T` check.
    fn as_any(&self) -> &dyn Any;

    /// The depth counter of a notification that mixes in `ViewportNotificationMixin`
    /// (`widgets/scroll_notification.rs`), for the viewport elements that increment it as
    /// the notification bubbles past them; `None` — the default — for every other
    /// notification.
    ///
    /// This is Dart's `notification is ViewportNotificationMixin`: a mixin is an interface,
    /// not a type, so an erased notification answers the check from its own vtable rather
    /// than through [`as_any`](Self::as_any). The counter is a [`Cell`] because a
    /// notification bubbles as `&dyn Notification`.
    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        None
    }

    /// The notification as a `ScrollNotification` (`widgets/scroll_notification.rs`), for a
    /// `NotificationListener<dyn ScrollNotification>`; `None` — the default — for every
    /// other notification.
    ///
    /// This is Dart's `notification is ScrollNotification`, answered from the vtable as
    /// [`viewport_depth`](Self::viewport_depth) is, because the family is a trait.
    fn as_scroll_notification(&self) -> Option<&dyn ScrollNotification> {
        None
    }

    /// Start bubbling this notification at the given build context.
    ///
    /// The notification will be delivered to any `NotificationListener` widgets with the
    /// appropriate type parameters that are ancestors of the given [`BuildContext`]. If the
    /// [`BuildContext`] is null, the notification is not dispatched.
    fn dispatch(&self, app: &mut App, target: Option<BuildContext>)
    where
        Self: Sized,
    {
        if let Some(target) = target {
            target.dispatch_notification(app, self);
        }
    }
}

/// Dart's `_NotificationNode`: one notifiable element and the chain above it.
pub struct NotificationNode {
    parent: Option<Rc<NotificationNode>>,
    current: AnyElement,
}

impl NotificationNode {
    /// A node for `current`, chained under its nearest notifiable ancestor.
    pub fn new(parent: Option<Rc<NotificationNode>>, current: AnyElement) -> NotificationNode {
        NotificationNode { parent, current }
    }

    fn dispatch_notification(&self, app: &mut App, notification: &dyn Notification) {
        if self.current.on_notification(app, notification) {
            return;
        }
        if let Some(parent) = &self.parent {
            parent.dispatch_notification(app, notification);
        }
    }
}

/// The fields of Dart's `Element` base class.
pub struct ElementData {
    parent: Option<AnyElement>,
    slot: Option<Slot>,
    depth: u32,
    widget: Option<WidgetRef>,
    owner: Option<Handle<BuildOwner>>,
    pub(crate) parent_build_scope: Option<Handle<BuildScope>>,
    lifecycle: ElementLifecycle,
    dirty: bool,
    pub(crate) in_dirty_list: bool,
    inherited_elements: Option<Rc<HashMap<TypeId, AnyElement>>>,
    /// Dart's `_notificationTree`: the nearest notifiable ancestor chain.
    notification_tree: Option<Rc<NotificationNode>>,
    dependencies: Option<HashSet<AnyElement>>,
    had_unsatisfied_dependencies: bool,
    debug_built_once: bool,
}

impl ElementData {
    /// Creates an element's bookkeeping for the given widget.
    pub fn new(widget: WidgetRef) -> ElementData {
        ElementData {
            parent: None,
            slot: None,
            depth: 0,
            widget: Some(widget),
            owner: None,
            parent_build_scope: None,
            lifecycle: ElementLifecycle::Initial,
            dirty: true,
            in_dirty_list: false,
            inherited_elements: None,
            notification_tree: None,
            dependencies: None,
            had_unsatisfied_dependencies: false,
            debug_built_once: false,
        }
    }

    /// The configuration for this element, as a shared reference.
    pub fn widget_ref(&self) -> &WidgetRef {
        self.widget.as_ref().expect("the element is unmounted")
    }
}

/// The accessors [`Element`] asks for, for a struct whose bag is the field `element`.
#[macro_export]
macro_rules! element_accessors {
    () => {
        fn element_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::ElementData {
            &app.get(self).element
        }

        fn element_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::ElementData {
            &mut app.get_mut(self).element
        }
    };
}

/// An instantiation of a `Widget` at a particular location in the tree.
///
/// Widgets describe how to configure a subtree but the same widget can be used to configure
/// multiple subtrees simultaneously because widgets are immutable. An [`Element`] represents
/// the use of a widget to configure a specific location in the tree. Over time, the widget
/// associated with a given element can change, for example, if the parent widget rebuilds
/// and creates a new widget for this location.
///
/// The overridable methods default to Dart's base bodies; an override that must run the
/// base as well calls it through [`ElementBase`] (`ElementBase::mount(self, ..)`), where
/// Dart calls `super`.
pub trait Element: Sized + 'static {
    /// Dart's base-class fields, held under the field `element`
    /// ([`element_accessors!`](crate::element_accessors)).
    fn element_data(self: Handle<Self>, app: &App) -> &ElementData;

    /// See [`element_data`](Self::element_data).
    fn element_data_mut(self: Handle<Self>, app: &mut App) -> &mut ElementData;

    /// The type-erased handle to this element.
    fn as_element(self: Handle<Self>) -> AnyElement {
        AnyElement {
            id: self.id(),
            vtable: const { &ElementVTable::of::<Self>() },
        }
    }

    /// Calls the argument for each child. Must be overridden by subclasses that support
    /// having children.
    ///
    /// There is no guaranteed order in which the children will be visited, though it should
    /// be consistent over time.
    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        let _ = (self, app, visitor);
    }

    /// Returns the child of this [`Element`] that will insert a `RenderObject` into an
    /// ancestor of this Element to construct the render tree.
    ///
    /// Returns `None` if this Element doesn't have any children who need to attach a
    /// `RenderObject` to an ancestor of this [`Element`]. A `RenderObjectElement` will
    /// therefore return `None` because its children insert their `RenderObject`s into the
    /// `RenderObjectElement` itself and not into an ancestor of the `RenderObjectElement`.
    fn render_object_attaching_child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        ElementBase::render_object_attaching_child(self, app)
    }

    /// Add this element to the tree in the given slot of the given parent.
    ///
    /// The framework calls this function when a newly created element is added to the tree
    /// for the first time. Use this method to initialize state that depends on having a
    /// parent. State that is independent of the parent can more easily be initialized in the
    /// constructor.
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ElementBase::mount(self, app, parent, new_slot);
    }

    /// Change the widget used to configure this element.
    ///
    /// The framework calls this function when the parent wishes to use a different widget to
    /// configure this element. The new widget is guaranteed to have the same `runtimeType` as
    /// the old widget.
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ElementBase::update(self, app, new_widget);
    }

    /// Called by [`AnyElement::update_slot_for_child`] when the slot for a child has changed.
    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        ElementBase::update_slot(self, app, new_slot);
    }

    /// Add [`AnyElement::render_object`] to the render tree at the location specified by
    /// `new_slot`.
    ///
    /// The default implementation of this function calls
    /// [`attach_render_object`](Self::attach_render_object) recursively on each child. The
    /// `RenderObjectElement::attach_render_object` override does the actual work of adding
    /// [`AnyElement::render_object`] to the render tree.
    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        ElementBase::attach_render_object(self, app, new_slot);
    }

    /// Remove [`AnyElement::render_object`] from the render tree.
    ///
    /// The default implementation of this function calls
    /// [`detach_render_object`](Self::detach_render_object) recursively on each child. The
    /// `RenderObjectElement::detach_render_object` override does the actual work of removing
    /// [`AnyElement::render_object`] from the render tree.
    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        ElementBase::detach_render_object(self, app);
    }

    /// Remove the given child from the element's child list, in preparation for the child
    /// being reused elsewhere in the element tree.
    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        let _ = (self, app, child);
    }

    /// Transition from the "inactive" to the "active" lifecycle state.
    fn activate(self: Handle<Self>, app: &mut App) {
        ElementBase::activate(self, app);
    }

    /// Transition from the "active" to the "inactive" lifecycle state.
    fn deactivate(self: Handle<Self>, app: &mut App) {
        ElementBase::deactivate(self, app);
    }

    /// Called, in debug mode, after children have been deactivated.
    fn debug_deactivated(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Inactive);
    }

    /// Transition from the "inactive" to the "defunct" lifecycle state.
    fn unmount(self: Handle<Self>, app: &mut App) {
        ElementBase::unmount(self, app);
    }

    /// Cause the widget to update itself.
    ///
    /// Called by [`AnyElement::rebuild`] after the appropriate checks have been made.
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ElementBase::perform_rebuild(self, app);
    }

    /// Called when a dependency of this element changes.
    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        ElementBase::did_change_dependencies(self, app);
    }

    /// Dart's `_updateInheritance`: inherit the parent's inherited-element map.
    fn update_inheritance(self: Handle<Self>, app: &mut App) {
        ElementBase::update_inheritance(self, app);
    }

    /// Called in [`mount`](Self::mount) and [`activate`](Self::activate) to register this
    /// element in the notification tree.
    ///
    /// This method is only exposed so that notifiable elements can be implemented (Dart's
    /// `NotifiableElementMixin`): an element that wishes to respond to notifications
    /// overrides it to insert a node for itself, and [`on_notification`](Self::on_notification)
    /// to handle them.
    ///
    /// See also:
    ///   * `NotificationListener`, a widget that allows listening to notifications.
    fn attach_notification_tree(self: Handle<Self>, app: &mut App) {
        ElementBase::attach_notification_tree(self, app);
    }

    /// The [`BuildScope`] whose dirty list this element is in.
    ///
    /// The [`BuildOwner`] builds the dirty elements of one scope per `build_scope` call;
    /// an element that wants its own (a `LayoutBuilder`, whose subtree is built during
    /// layout) overrides this.
    fn build_scope(self: Handle<Self>, app: &App) -> Handle<BuildScope> {
        ElementBase::build_scope(self, app)
    }

    /// Marks the element as dirty and adds it to the global list of widgets to rebuild in
    /// the next frame.
    ///
    /// Since it is inefficient to build an element twice in one frame, applications and
    /// widgets should be structured so as to only mark widgets dirty during event handlers
    /// before the frame begins, not during the build itself.
    fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        ElementBase::mark_needs_build(self, app);
    }

    /// Called when a notification of the appropriate type arrives at this location in the
    /// tree (Dart's `NotifiableElementMixin.onNotification`).
    ///
    /// Return true to cancel the notification bubbling. Return false to allow the
    /// notification to continue to be dispatched to further ancestors. Only elements that
    /// override [`attach_notification_tree`](Self::attach_notification_tree) are asked.
    fn on_notification(self: Handle<Self>, app: &mut App, notification: &dyn Notification) -> bool {
        let _ = (app, notification);
        false
    }

    /// Whether the child in the provided `slot` (or one of its descendants) must insert a
    /// `RenderObject` into its ancestor `RenderObjectElement` by calling
    /// `RenderObjectElement::insert_render_object_child` on it.
    fn debug_expects_render_object_for_slot(
        self: Handle<Self>,
        app: &App,
        slot: Option<&Slot>,
    ) -> bool {
        let _ = (self, app, slot);
        true
    }

    /// The render object at (or below) this location in the tree.
    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        ElementBase::render_object(self, app)
    }

    /// Create an element for the given widget and add it as a child of this element in the
    /// given slot.
    fn inflate_widget(
        self: Handle<Self>,
        app: &mut App,
        new_widget: WidgetRef,
        new_slot: Option<Slot>,
    ) -> AnyElement {
        ElementBase::inflate_widget(self, app, new_widget, new_slot)
    }

    /// Whether this element is in the middle of building.
    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        let _ = (self, app);
        false
    }

    // ---- Dart's `is` checks and the members behind them ----

    /// Dart's `this is RenderObjectElement`.
    const IS_RENDER_OBJECT_ELEMENT: bool = false;

    /// `RenderObjectElement.insertRenderObjectChild`.
    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let _ = (self, app, child, slot);
        unreachable!("not a RenderObjectElement");
    }

    /// `RenderObjectElement.moveRenderObjectChild`.
    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        let _ = (self, app, child, old_slot, new_slot);
        unreachable!("not a RenderObjectElement");
    }

    /// `RenderObjectElement.removeRenderObjectChild`.
    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let _ = (self, app, child, slot);
        unreachable!("not a RenderObjectElement");
    }

    /// `RenderObjectElement._updateParentData`: apply a parent-data widget to this element's
    /// render object.
    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        let _ = (self, app, parent_data_element);
        unreachable!("not a RenderObjectElement");
    }

    /// Dart's `this is ParentDataElement`.
    const IS_PARENT_DATA_ELEMENT: bool = false;

    /// `ParentDataWidget.applyParentData` through the element that holds the widget.
    fn apply_parent_data(
        self: Handle<Self>,
        app: &mut App,
        render_object: AnyRenderObject,
    ) -> bool {
        let _ = (self, app, render_object);
        unreachable!("not a ParentDataElement");
    }

    /// Dart's `this is InheritedElement`.
    const IS_INHERITED_ELEMENT: bool = false;

    /// `InheritedElement.updateDependencies`.
    fn update_dependencies(
        self: Handle<Self>,
        app: &mut App,
        dependent: AnyElement,
        aspect: Option<Rc<dyn Any>>,
    ) {
        let _ = (self, app, dependent, aspect);
        unreachable!("not an InheritedElement");
    }

    /// `InheritedElement.removeDependent`.
    fn remove_dependent(self: Handle<Self>, app: &mut App, dependent: AnyElement) {
        let _ = (self, app, dependent);
        unreachable!("not an InheritedElement");
    }

    /// Dart's `this is StatefulElement && state is T`: the state's type and arena id.
    fn state_handle_id(self: Handle<Self>, app: &App) -> Option<(TypeId, HandleId)> {
        let _ = (self, app);
        None
    }
}

/// The vtable of an erased [`AnyElement`]: one `&'static` table per element type.
pub(crate) struct ElementVTable {
    pub element_data: fn(&App, HandleId) -> &ElementData,
    pub element_data_mut: fn(&mut App, HandleId) -> &mut ElementData,
    pub visit_children: fn(&App, HandleId, &mut dyn FnMut(AnyElement)),
    pub render_object_attaching_child: fn(&App, HandleId) -> Option<AnyElement>,
    pub mount: fn(&mut App, HandleId, Option<AnyElement>, Option<Slot>),
    pub update: fn(&mut App, HandleId, WidgetRef),
    pub update_slot: fn(&mut App, HandleId, Option<Slot>),
    pub attach_render_object: fn(&mut App, HandleId, Option<Slot>),
    pub detach_render_object: fn(&mut App, HandleId),
    pub forget_child: fn(&mut App, HandleId, AnyElement),
    pub activate: fn(&mut App, HandleId),
    pub deactivate: fn(&mut App, HandleId),
    pub debug_deactivated: fn(&mut App, HandleId),
    pub unmount: fn(&mut App, HandleId),
    pub perform_rebuild: fn(&mut App, HandleId),
    pub did_change_dependencies: fn(&mut App, HandleId),
    pub update_inheritance: fn(&mut App, HandleId),
    pub attach_notification_tree: fn(&mut App, HandleId),
    pub build_scope: fn(&App, HandleId) -> Handle<BuildScope>,
    pub mark_needs_build: fn(&mut App, HandleId),
    pub on_notification: fn(&mut App, HandleId, &dyn Notification) -> bool,
    pub debug_expects_render_object_for_slot: fn(&App, HandleId, Option<&Slot>) -> bool,
    pub render_object: fn(&App, HandleId) -> Option<AnyRenderObject>,
    pub inflate_widget: fn(&mut App, HandleId, WidgetRef, Option<Slot>) -> AnyElement,
    pub debug_doing_build: fn(&App, HandleId) -> bool,
    pub is_render_object_element: bool,
    pub insert_render_object_child: fn(&mut App, HandleId, AnyRenderObject, Option<Slot>),
    pub move_render_object_child:
        fn(&mut App, HandleId, AnyRenderObject, Option<Slot>, Option<Slot>),
    pub remove_render_object_child: fn(&mut App, HandleId, AnyRenderObject, Option<Slot>),
    pub update_parent_data: fn(&mut App, HandleId, AnyElement),
    pub is_parent_data_element: bool,
    pub apply_parent_data: fn(&mut App, HandleId, AnyRenderObject) -> bool,
    pub is_inherited_element: bool,
    pub update_dependencies: fn(&mut App, HandleId, AnyElement, Option<Rc<dyn Any>>),
    pub remove_dependent: fn(&mut App, HandleId, AnyElement),
    pub state_handle_id: fn(&App, HandleId) -> Option<(TypeId, HandleId)>,
}

/// The typed handle for an erased id. Free: nothing is looked up; `App::get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl ElementVTable {
    pub(crate) const fn of<T: Element>() -> ElementVTable {
        ElementVTable {
            element_data: |app, id| T::element_data(resolve(id), app),
            element_data_mut: |app, id| T::element_data_mut(resolve(id), app),
            visit_children: |app, id, visitor| T::visit_children(resolve(id), app, visitor),
            render_object_attaching_child: |app, id| {
                T::render_object_attaching_child(resolve(id), app)
            },
            mount: |app, id, parent, slot| T::mount(resolve(id), app, parent, slot),
            update: |app, id, widget| T::update(resolve(id), app, widget),
            update_slot: |app, id, slot| T::update_slot(resolve(id), app, slot),
            attach_render_object: |app, id, slot| T::attach_render_object(resolve(id), app, slot),
            detach_render_object: |app, id| T::detach_render_object(resolve(id), app),
            forget_child: |app, id, child| T::forget_child(resolve(id), app, child),
            activate: |app, id| T::activate(resolve(id), app),
            deactivate: |app, id| T::deactivate(resolve(id), app),
            debug_deactivated: |app, id| T::debug_deactivated(resolve(id), app),
            unmount: |app, id| T::unmount(resolve(id), app),
            perform_rebuild: |app, id| T::perform_rebuild(resolve(id), app),
            did_change_dependencies: |app, id| T::did_change_dependencies(resolve(id), app),
            update_inheritance: |app, id| T::update_inheritance(resolve(id), app),
            attach_notification_tree: |app, id| T::attach_notification_tree(resolve(id), app),
            build_scope: |app, id| T::build_scope(resolve(id), app),
            mark_needs_build: |app, id| T::mark_needs_build(resolve(id), app),
            on_notification: |app, id, notification| {
                T::on_notification(resolve(id), app, notification)
            },
            debug_expects_render_object_for_slot: |app, id, slot| {
                T::debug_expects_render_object_for_slot(resolve(id), app, slot)
            },
            render_object: |app, id| T::render_object(resolve(id), app),
            inflate_widget: |app, id, widget, slot| {
                T::inflate_widget(resolve(id), app, widget, slot)
            },
            debug_doing_build: |app, id| T::debug_doing_build(resolve(id), app),
            is_render_object_element: T::IS_RENDER_OBJECT_ELEMENT,
            insert_render_object_child: |app, id, child, slot| {
                T::insert_render_object_child(resolve(id), app, child, slot)
            },
            move_render_object_child: |app, id, child, old_slot, new_slot| {
                T::move_render_object_child(resolve(id), app, child, old_slot, new_slot)
            },
            remove_render_object_child: |app, id, child, slot| {
                T::remove_render_object_child(resolve(id), app, child, slot)
            },
            update_parent_data: |app, id, element| T::update_parent_data(resolve(id), app, element),
            is_parent_data_element: T::IS_PARENT_DATA_ELEMENT,
            apply_parent_data: |app, id, render_object| {
                T::apply_parent_data(resolve(id), app, render_object)
            },
            is_inherited_element: T::IS_INHERITED_ELEMENT,
            update_dependencies: |app, id, dependent, aspect| {
                T::update_dependencies(resolve(id), app, dependent, aspect)
            },
            remove_dependent: |app, id, dependent| T::remove_dependent(resolve(id), app, dependent),
            state_handle_id: |app, id| T::state_handle_id(resolve(id), app),
        }
    }
}

/// Erased `Element`: one identity and a static vtable, like `AnyRenderObject`.
///
/// Equality is Dart's `==` on an element, which is identity.
#[derive(Clone, Copy)]
pub struct AnyElement {
    id: HandleId,
    vtable: &'static ElementVTable,
}

impl PartialEq for AnyElement {
    fn eq(&self, other: &AnyElement) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyElement {}

impl Hash for AnyElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Element({:?})", self.id)
    }
}

impl AnyElement {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `element as T`: the typed handle when this element is a `T`, else `None`.
    pub fn downcast<T: Element>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    pub(crate) fn data(self, app: &App) -> &ElementData {
        (self.vtable.element_data)(app, self.id)
    }

    pub(crate) fn data_mut(self, app: &mut App) -> &mut ElementData {
        (self.vtable.element_data_mut)(app, self.id)
    }

    // ---- fields ----

    /// The configuration for this element.
    ///
    /// Avoid overriding this field on `Element` subtypes.
    pub fn widget(self, app: &App) -> &WidgetRef {
        self.data(app).widget_ref()
    }

    /// Returns true if the Element is active.
    pub fn mounted(self, app: &App) -> bool {
        self.data(app).widget.is_some()
    }

    /// The parent of this element, or `None` at the root.
    pub fn parent(self, app: &App) -> Option<AnyElement> {
        self.data(app).parent
    }

    /// Information set by parent to define where this child fits in its parent's child list.
    pub fn slot(self, app: &App) -> Option<Slot> {
        self.data(app).slot.clone()
    }

    /// An integer that is guaranteed to be greater than the parent's, if any. The element at
    /// the root of the tree must have a depth greater than 0.
    pub fn depth(self, app: &App) -> u32 {
        debug_assert!(
            self.lifecycle(app) != ElementLifecycle::Initial,
            "Depth is only available when element has been mounted."
        );
        self.data(app).depth
    }

    /// The object that manages the lifecycle of this element.
    pub fn owner(self, app: &App) -> Option<Handle<BuildOwner>> {
        self.data(app).owner
    }

    /// The [`BuildScope`] whose dirty list this element is in.
    pub fn build_scope(self, app: &App) -> Handle<BuildScope> {
        (self.vtable.build_scope)(app, self.id)
    }

    /// Where this element is in its lifecycle.
    pub fn lifecycle(self, app: &App) -> ElementLifecycle {
        self.data(app).lifecycle
    }

    /// Whether the element is active (in the tree).
    pub fn debug_is_active(self, app: &App) -> bool {
        self.lifecycle(app) == ElementLifecycle::Active
    }

    /// Whether the element is defunct (unmounted).
    pub fn debug_is_defunct(self, app: &App) -> bool {
        self.lifecycle(app) == ElementLifecycle::Defunct
    }

    /// Returns true if the element has been marked as needing rebuilding.
    ///
    /// The flag is true when the element is first created and after
    /// [`mark_needs_build`](Self::mark_needs_build) has been called. The flag is reset to
    /// false in the [`perform_rebuild`](Self::perform_rebuild) implementation.
    pub fn dirty(self, app: &App) -> bool {
        self.data(app).dirty
    }

    pub(crate) fn in_dirty_list(self, app: &App) -> bool {
        self.data(app).in_dirty_list
    }

    /// Whether this element is in the middle of building.
    pub fn debug_doing_build(self, app: &App) -> bool {
        (self.vtable.debug_doing_build)(app, self.id)
    }

    /// The children, in visit order.
    pub fn children(self, app: &App) -> Vec<AnyElement> {
        let mut children = Vec::new();
        self.visit_children(app, &mut |child| children.push(child));
        children
    }

    // ---- virtual dispatch ----

    /// See [`Element::visit_children`].
    pub fn visit_children(self, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        (self.vtable.visit_children)(app, self.id, visitor);
    }

    /// See [`Element::render_object_attaching_child`].
    pub fn render_object_attaching_child(self, app: &App) -> Option<AnyElement> {
        (self.vtable.render_object_attaching_child)(app, self.id)
    }

    /// See [`Element::mount`].
    pub fn mount(self, app: &mut App, parent: Option<AnyElement>, new_slot: Option<Slot>) {
        (self.vtable.mount)(app, self.id, parent, new_slot);
    }

    /// See [`Element::update`].
    pub fn update(self, app: &mut App, new_widget: WidgetRef) {
        (self.vtable.update)(app, self.id, new_widget);
    }

    /// See [`Element::update_slot`].
    pub fn update_slot(self, app: &mut App, new_slot: Option<Slot>) {
        (self.vtable.update_slot)(app, self.id, new_slot);
    }

    /// See [`Element::attach_render_object`].
    pub fn attach_render_object(self, app: &mut App, new_slot: Option<Slot>) {
        (self.vtable.attach_render_object)(app, self.id, new_slot);
    }

    /// See [`Element::detach_render_object`].
    pub fn detach_render_object(self, app: &mut App) {
        (self.vtable.detach_render_object)(app, self.id);
    }

    /// See [`Element::forget_child`].
    pub fn forget_child(self, app: &mut App, child: AnyElement) {
        (self.vtable.forget_child)(app, self.id, child);
    }

    /// See [`Element::activate`].
    pub fn activate(self, app: &mut App) {
        (self.vtable.activate)(app, self.id);
    }

    /// See [`Element::deactivate`].
    pub fn deactivate(self, app: &mut App) {
        (self.vtable.deactivate)(app, self.id);
    }

    /// See [`Element::debug_deactivated`].
    pub fn debug_deactivated(self, app: &mut App) {
        (self.vtable.debug_deactivated)(app, self.id);
    }

    /// See [`Element::unmount`].
    pub fn unmount(self, app: &mut App) {
        (self.vtable.unmount)(app, self.id);
    }

    /// See [`Element::perform_rebuild`].
    pub fn perform_rebuild(self, app: &mut App) {
        (self.vtable.perform_rebuild)(app, self.id);
    }

    /// See [`Element::did_change_dependencies`].
    pub fn did_change_dependencies(self, app: &mut App) {
        (self.vtable.did_change_dependencies)(app, self.id);
    }

    /// See [`Element::update_inheritance`].
    pub fn update_inheritance(self, app: &mut App) {
        (self.vtable.update_inheritance)(app, self.id);
    }

    /// See [`Element::attach_notification_tree`].
    pub fn attach_notification_tree(self, app: &mut App) {
        (self.vtable.attach_notification_tree)(app, self.id);
    }

    /// See [`Element::on_notification`].
    pub fn on_notification(self, app: &mut App, notification: &dyn Notification) -> bool {
        (self.vtable.on_notification)(app, self.id, notification)
    }

    /// Start bubbling this notification at the given build context.
    ///
    /// The notification will be delivered to any `NotificationListener` widgets with the
    /// appropriate type parameters that are ancestors of the given [`BuildContext`].
    pub fn dispatch_notification(self, app: &mut App, notification: &dyn Notification) {
        let tree = self.data(app).notification_tree.clone();
        if let Some(tree) = tree {
            tree.dispatch_notification(app, notification);
        }
    }

    /// See [`Element::debug_expects_render_object_for_slot`].
    pub fn debug_expects_render_object_for_slot(self, app: &App, slot: Option<&Slot>) -> bool {
        (self.vtable.debug_expects_render_object_for_slot)(app, self.id, slot)
    }

    /// The render object at (or below) this location in the tree.
    ///
    /// If this object is a `RenderObjectElement`, the render object is the one at this
    /// location in the tree. Otherwise, this getter will walk down the tree until it finds a
    /// `RenderObjectElement`.
    ///
    /// Some locations in the tree are not backed by a render object. In those cases, this
    /// getter returns `None`.
    pub fn render_object(self, app: &App) -> Option<AnyRenderObject> {
        (self.vtable.render_object)(app, self.id)
    }

    /// See [`Element::inflate_widget`].
    pub fn inflate_widget(
        self,
        app: &mut App,
        new_widget: WidgetRef,
        new_slot: Option<Slot>,
    ) -> AnyElement {
        (self.vtable.inflate_widget)(app, self.id, new_widget, new_slot)
    }

    /// Dart's `this is RenderObjectElement`.
    pub fn is_render_object_element(self) -> bool {
        self.vtable.is_render_object_element
    }

    /// See [`Element::insert_render_object_child`].
    pub fn insert_render_object_child(
        self,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        (self.vtable.insert_render_object_child)(app, self.id, child, slot);
    }

    /// See [`Element::move_render_object_child`].
    pub fn move_render_object_child(
        self,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        (self.vtable.move_render_object_child)(app, self.id, child, old_slot, new_slot);
    }

    /// See [`Element::remove_render_object_child`].
    pub fn remove_render_object_child(
        self,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        (self.vtable.remove_render_object_child)(app, self.id, child, slot);
    }

    /// See [`Element::update_parent_data`].
    pub fn update_parent_data(self, app: &mut App, parent_data_element: AnyElement) {
        (self.vtable.update_parent_data)(app, self.id, parent_data_element);
    }

    /// Dart's `this is ParentDataElement`.
    pub fn is_parent_data_element(self) -> bool {
        self.vtable.is_parent_data_element
    }

    /// See [`Element::apply_parent_data`].
    pub fn apply_parent_data(self, app: &mut App, render_object: AnyRenderObject) -> bool {
        (self.vtable.apply_parent_data)(app, self.id, render_object)
    }

    /// Dart's `this is InheritedElement`.
    pub fn is_inherited_element(self) -> bool {
        self.vtable.is_inherited_element
    }

    /// See [`Element::update_dependencies`].
    pub fn update_dependencies(
        self,
        app: &mut App,
        dependent: AnyElement,
        aspect: Option<Rc<dyn Any>>,
    ) {
        (self.vtable.update_dependencies)(app, self.id, dependent, aspect);
    }

    /// See [`Element::remove_dependent`].
    pub fn remove_dependent(self, app: &mut App, dependent: AnyElement) {
        (self.vtable.remove_dependent)(app, self.id, dependent);
    }

    /// The typed state handle when this is a `StatefulElement` whose state is an `S`.
    pub fn state_handle<S: State>(self, app: &App) -> Option<Handle<S>> {
        let (type_id, id) = (self.vtable.state_handle_id)(app, self.id)?;
        (type_id == TypeId::of::<S>()).then(|| Handle::from_id(id))
    }

    // ---- Dart's `Element` concrete members ----

    /// Update the given child with the given new configuration.
    ///
    /// This method is the core of the widgets system. It is called each time we are to add,
    /// update, or remove a child based on an updated configuration.
    ///
    /// The `new_slot` argument specifies the new value for this element's
    /// [`slot`](Self::slot).
    ///
    /// If the `child` is `None`, and the `new_widget` is not `None`, then we have a new child
    /// for which we need to create an [`Element`], configured with `new_widget`.
    ///
    /// If the `new_widget` is `None`, and the `child` is not `None`, then we need to remove
    /// it because it no longer has a configuration.
    ///
    /// If neither are `None`, then we need to update the `child`'s configuration to be the
    /// new configuration given by `new_widget`. If `new_widget` can be given to the existing
    /// child (as determined by `can_update`), then it is so given. Otherwise, the old child
    /// needs to be disposed and a new child created for the new configuration.
    ///
    /// If both are `None`, then we don't have a child and won't have a child, so we do
    /// nothing.
    ///
    /// The [`update_child`](Self::update_child) method returns the new child, if it had to
    /// create one, or the child that was passed in, if it just had to update the child, or
    /// `None`, if it removed the child and did not replace it.
    pub fn update_child(
        self,
        app: &mut App,
        child: Option<AnyElement>,
        new_widget: Option<WidgetRef>,
        new_slot: Option<Slot>,
    ) -> Option<AnyElement> {
        let Some(new_widget) = new_widget else {
            if let Some(child) = child {
                self.deactivate_child(app, child);
            }
            return None;
        };
        let new_child = match child {
            Some(child) => {
                let has_same_superclass = child.widget(app).kind() == new_widget.kind();
                if has_same_superclass && same_widget(child.widget(app), &new_widget) {
                    if child.slot(app) != new_slot {
                        self.update_slot_for_child(app, child, new_slot);
                    }
                    child
                } else if has_same_superclass && can_update(&**child.widget(app), &*new_widget) {
                    if child.slot(app) != new_slot {
                        self.update_slot_for_child(app, child, new_slot);
                    }
                    child.update(app, new_widget.clone());
                    debug_assert!(same_widget(child.widget(app), &new_widget));
                    child
                } else {
                    self.deactivate_child(app, child);
                    debug_assert!(child.parent(app).is_none());
                    self.inflate_widget(app, new_widget, new_slot)
                }
            }
            None => self.inflate_widget(app, new_widget, new_slot),
        };
        Some(new_child)
    }

    /// Updates the children of this element to use new widgets.
    ///
    /// Attempts to update the given old children list using the given new widgets, removing
    /// obsolete elements and introducing new ones as necessary, and then returns the new
    /// child list.
    ///
    /// During this function the `old_children` list must not be modified. If the caller
    /// wishes to remove elements from `old_children` reentrantly while this function is on
    /// the stack, the caller can supply a `forgotten_children` argument, which can be
    /// modified while this function is on the stack. Whenever this function reads from
    /// `old_children`, this function first checks whether the child is in
    /// `forgotten_children`. If it is, the function acts as if the child was not in
    /// `old_children`.
    ///
    /// `forgotten_children` is a predicate so that it reads the caller's set live (in the
    /// arena, where a reentrant `forget_child` writes it) rather than a snapshot.
    ///
    /// This function is a convenience wrapper around [`update_child`](Self::update_child),
    /// which updates each individual child. If `slots` is non-`None`, the value for the
    /// `new_slot` argument of [`update_child`](Self::update_child) is retrieved from that
    /// list using the index that the currently processed `child` corresponds to in the
    /// `new_widgets` list (`new_widgets` and `slots` must have the same length). If `slots`
    /// is `None`, an [`IndexedSlot`] is used as the value for the `new_slot` argument. In
    /// that case, [`IndexedSlot::index`] is set to the index that the currently processed
    /// `child` corresponds to in the `new_widgets` list and [`IndexedSlot::value`] is set to
    /// the [`Element`] of the previous widget in that list (or `None` if it is the first
    /// child).
    pub fn update_children(
        self,
        app: &mut App,
        old_children: &[AnyElement],
        new_widgets: &[WidgetRef],
        forgotten_children: Option<ForgottenChildren<'_>>,
        slots: Option<&[Option<Slot>]>,
    ) -> Vec<AnyElement> {
        debug_assert!(slots.is_none_or(|slots| slots.len() == new_widgets.len()));
        let replace_with_null_if_forgotten = |app: &App, child: AnyElement| -> Option<AnyElement> {
            let forgotten = forgotten_children.is_some_and(|forgotten| forgotten(app, child));
            (!forgotten).then_some(child)
        };
        let slot_for =
            |new_child_index: usize, previous_child: Option<AnyElement>| -> Option<Slot> {
                match slots {
                    Some(slots) => slots[new_child_index].clone(),
                    None => Some(Slot::Indexed(IndexedSlot {
                        index: new_child_index,
                        value: previous_child,
                    })),
                }
            };

        // This attempts to diff the new child list (newWidgets) with
        // the old child list (oldChildren), and produce a new list of elements to
        // be the new list of child elements of this element. The called of this
        // method is expected to update this render object accordingly.

        // The cases it tries to optimize for are:
        //  - the old list is empty
        //  - the lists are identical
        //  - there is an insertion or removal of one or more widgets in
        //    only one place in the list
        // If a widget with a key is in both lists, it will be synced.
        // Widgets without keys might be synced but there is no guarantee.

        // The general approach is to sync the entire new list backwards, as follows:
        // 1. Walk the lists from the top, invoking updateChild on each element.
        // 2. Walk the lists from the bottom, without syncing, and simply track the
        //    number of matching elements at the end.
        // 3. Walk the middle of the old list, and collect the keyed children in a map.
        // 4. Walk the middle of the new list, syncing with the keyed old children when
        //    keys match, otherwise inflating new elements.
        // 5. Walk the bottom of the lists, syncing the elements that matched at the end.
        // 6. Deactivate any old children that were not synced.
        let mut new_children_top = 0usize;
        let mut old_children_top = 0usize;
        let mut new_children_bottom = new_widgets.len() as isize - 1;
        let mut old_children_bottom = old_children.len() as isize - 1;

        let mut new_children: Vec<Option<AnyElement>> = vec![None; new_widgets.len()];
        let mut previous_child: Option<AnyElement> = None;

        // Update the top of the list.
        while (old_children_top as isize <= old_children_bottom)
            && (new_children_top as isize <= new_children_bottom)
        {
            let old_child = replace_with_null_if_forgotten(app, old_children[old_children_top]);
            let new_widget = &new_widgets[new_children_top];
            debug_assert!(
                old_child.is_none_or(|old| old.lifecycle(app) == ElementLifecycle::Active)
            );
            let Some(old_child) = old_child else {
                break;
            };
            if !can_update(&**old_child.widget(app), &**new_widget) {
                break;
            }
            let new_child = self
                .update_child(
                    app,
                    Some(old_child),
                    Some(new_widget.clone()),
                    slot_for(new_children_top, previous_child),
                )
                .expect("a widget yields a child");
            debug_assert!(new_child.lifecycle(app) == ElementLifecycle::Active);
            new_children[new_children_top] = Some(new_child);
            previous_child = Some(new_child);
            new_children_top += 1;
            old_children_top += 1;
        }

        // Scan the bottom of the list.
        while (old_children_top as isize <= old_children_bottom)
            && (new_children_top as isize <= new_children_bottom)
        {
            let old_child =
                replace_with_null_if_forgotten(app, old_children[old_children_bottom as usize]);
            let new_widget = &new_widgets[new_children_bottom as usize];
            debug_assert!(
                old_child.is_none_or(|old| old.lifecycle(app) == ElementLifecycle::Active)
            );
            let Some(old_child) = old_child else {
                break;
            };
            if !can_update(&**old_child.widget(app), &**new_widget) {
                break;
            }
            old_children_bottom -= 1;
            new_children_bottom -= 1;
        }

        // Scan the old children in the middle of the list.
        let have_old_children = old_children_top as isize <= old_children_bottom;
        let mut old_keyed_children: HashMap<KeyIdentity, AnyElement> = HashMap::new();
        if have_old_children {
            while old_children_top as isize <= old_children_bottom {
                let old_child = replace_with_null_if_forgotten(app, old_children[old_children_top]);
                debug_assert!(
                    old_child.is_none_or(|old| old.lifecycle(app) == ElementLifecycle::Active)
                );
                if let Some(old_child) = old_child {
                    match old_child.widget(app).key().cloned() {
                        Some(key) => {
                            old_keyed_children.insert(KeyIdentity(key), old_child);
                        }
                        None => self.deactivate_child(app, old_child),
                    }
                }
                old_children_top += 1;
            }
        }

        // Update the middle of the list.
        while new_children_top as isize <= new_children_bottom {
            let new_widget = &new_widgets[new_children_top];
            let mut old_child: Option<AnyElement> = None;
            if have_old_children && let Some(key) = new_widget.key() {
                let identity = KeyIdentity(key.clone());
                if let Some(candidate) = old_keyed_children.get(&identity).copied()
                    && can_update(&**candidate.widget(app), &**new_widget)
                {
                    // we found a match!
                    // remove it from oldKeyedChildren so we don't unsync it later
                    old_keyed_children.remove(&identity);
                    old_child = Some(candidate);
                }
            }
            debug_assert!(
                old_child.is_none_or(|old| can_update(&**old.widget(app), &**new_widget))
            );
            let new_child = self
                .update_child(
                    app,
                    old_child,
                    Some(new_widget.clone()),
                    slot_for(new_children_top, previous_child),
                )
                .expect("a widget yields a child");
            debug_assert!(new_child.lifecycle(app) == ElementLifecycle::Active);
            debug_assert!(old_child.is_none_or(
                |old| old == new_child || old.lifecycle(app) != ElementLifecycle::Active
            ));
            new_children[new_children_top] = Some(new_child);
            previous_child = Some(new_child);
            new_children_top += 1;
        }

        // We've scanned the whole list.
        debug_assert!(old_children_top as isize == old_children_bottom + 1);
        debug_assert!(new_children_top as isize == new_children_bottom + 1);
        debug_assert!(
            new_widgets.len() - new_children_top == old_children.len() - old_children_top
        );
        new_children_bottom = new_widgets.len() as isize - 1;
        old_children_bottom = old_children.len() as isize - 1;

        // Update the bottom of the list.
        while (old_children_top as isize <= old_children_bottom)
            && (new_children_top as isize <= new_children_bottom)
        {
            let old_child = old_children[old_children_top];
            debug_assert!(replace_with_null_if_forgotten(app, old_child).is_some());
            debug_assert!(old_child.lifecycle(app) == ElementLifecycle::Active);
            let new_widget = &new_widgets[new_children_top];
            debug_assert!(can_update(&**old_child.widget(app), &**new_widget));
            let new_child = self
                .update_child(
                    app,
                    Some(old_child),
                    Some(new_widget.clone()),
                    slot_for(new_children_top, previous_child),
                )
                .expect("a widget yields a child");
            debug_assert!(new_child.lifecycle(app) == ElementLifecycle::Active);
            debug_assert!(
                old_child == new_child || old_child.lifecycle(app) != ElementLifecycle::Active
            );
            new_children[new_children_top] = Some(new_child);
            previous_child = Some(new_child);
            new_children_top += 1;
            old_children_top += 1;
        }

        // Clean up any of the remaining middle nodes from the old list.
        if have_old_children {
            for old_child in old_keyed_children.into_values() {
                if replace_with_null_if_forgotten(app, old_child).is_some() {
                    self.deactivate_child(app, old_child);
                }
            }
        }
        new_children
            .into_iter()
            .map(|child| child.expect("every slot was filled"))
            .collect()
    }

    /// Change the slot that the given child occupies in its parent.
    ///
    /// Called by `MultiChildRenderObjectElement`, and other `RenderObjectElement` subclasses
    /// that have multiple children, when child moves from one position to another in this
    /// element's child list.
    pub fn update_slot_for_child(self, app: &mut App, child: AnyElement, new_slot: Option<Slot>) {
        debug_assert!(self.lifecycle(app) == ElementLifecycle::Active);
        debug_assert!(child.parent(app) == Some(self));
        let mut element = Some(child);
        while let Some(current) = element {
            current.update_slot(app, new_slot.clone());
            element = current.render_object_attaching_child(app);
        }
    }

    fn update_depth(self, app: &mut App, parent_depth: u32) {
        let expected_depth = parent_depth + 1;
        if self.data(app).depth < expected_depth {
            self.data_mut(app).depth = expected_depth;
            for child in self.children(app) {
                child.update_depth(app, expected_depth);
            }
        }
    }

    fn update_build_scope_recursively(self, app: &mut App) {
        let parent_scope = self.parent(app).map(|parent| parent.build_scope(app));
        if self.data(app).parent_build_scope == parent_scope {
            return;
        }
        // Unset the _inDirtyList flag so this Element can be added to the dirty list
        // of the new build scope.
        let data = self.data_mut(app);
        data.in_dirty_list = false;
        data.parent_build_scope = parent_scope;
        for child in self.children(app) {
            child.update_build_scope_recursively(app);
        }
    }

    fn retake_inactive_element(
        self,
        app: &mut App,
        key: super::widget::GlobalKeyId,
        new_widget: &WidgetRef,
    ) -> Option<AnyElement> {
        // The "inactivity" of the element being retaken here may be forward-looking: if
        // we are taking an element with a GlobalKey from an element that currently has
        // it as a child, then we know that element will soon no longer have that
        // element as a child. The only way that assumption could be false is if the
        // global key is being duplicated, and we'll try to track that using the
        // _debugTrackElementThatWillNeedToBeRebuiltDueToGlobalKeyShenanigans call below.
        let owner = self.owner(app).expect("a mounted element has an owner");
        let element = owner.global_key_element(app, key)?;
        if !can_update(&**element.widget(app), &**new_widget) {
            return None;
        }
        if let Some(parent) = element.parent(app) {
            assert!(
                parent != self,
                "A GlobalKey was used multiple times inside one widget's child list. A \
                 GlobalKey can only be specified on one widget at a time in the widget tree."
            );
            parent.forget_child(app, element);
            parent.deactivate_child(app, element);
        }
        debug_assert!(element.parent(app).is_none());
        owner.inactive_remove(app, element);
        Some(element)
    }

    /// Move the given element to the list of inactive elements and detach its render object
    /// from the render tree.
    ///
    /// This method stops the given element from being a child of this element by detaching
    /// its render object from the render tree and moving the element to the list of inactive
    /// elements.
    ///
    /// This method (indirectly) calls [`deactivate`](Self::deactivate) on the child.
    ///
    /// The caller is responsible for removing the child from its child model. Typically
    /// [`deactivate_child`](Self::deactivate_child) is called by the element itself while it
    /// is updating its child model; however, during
    /// `GlobalKey` reparenting, the new parent proactively calls the old parent's
    /// [`deactivate_child`](Self::deactivate_child), first using [`forget_child`](Self::forget_child)
    /// to cause the old parent to update its child model.
    pub fn deactivate_child(self, app: &mut App, child: AnyElement) {
        debug_assert!(child.parent(app) == Some(self));
        child.data_mut(app).parent = None;
        child.detach_render_object(app);
        let owner = self.owner(app).expect("a mounted element has an owner");
        owner.inactive_add(app, child); // this eventually calls child.deactivate()
    }

    fn activate_with_parent(self, app: &mut App, parent: AnyElement, new_slot: Option<Slot>) {
        debug_assert!(self.lifecycle(app) == ElementLifecycle::Inactive);
        let parent_owner = parent.owner(app);
        let data = self.data_mut(app);
        data.parent = Some(parent);
        data.owner = parent_owner;
        let parent_depth = parent.depth(app);
        self.update_depth(app, parent_depth);
        self.update_build_scope_recursively(app);
        activate_recursively(app, self);
        self.attach_render_object(app, new_slot);
        debug_assert!(self.lifecycle(app) == ElementLifecycle::Active);
    }

    pub(crate) fn ensure_deactivated(self, app: &mut App) {
        let dependencies = self.data_mut(app).dependencies.take();
        if let Some(dependencies) = dependencies {
            for dependency in dependencies {
                dependency.remove_dependent(app, self);
            }
        }
        let data = self.data_mut(app);
        data.inherited_elements = None;
        data.lifecycle = ElementLifecycle::Inactive;
    }

    /// The current `RenderObject` for the widget. If the widget is a `RenderObjectWidget`,
    /// this is the render object that the widget created for itself. Otherwise, it is the
    /// render object of the first descendant `RenderObjectWidget`.
    ///
    /// This method will only return a valid result after the build phase is complete. It is
    /// therefore not valid to call this from a build method. It should only be called from
    /// interaction event handlers (e.g. gesture callbacks) or layout or paint callbacks. It
    /// is also not valid to call if [`mounted`](Self::mounted) is false.
    pub fn find_render_object(self, app: &App) -> Option<AnyRenderObject> {
        debug_assert!(
            self.lifecycle(app) == ElementLifecycle::Active,
            "Cannot get renderObject of inactive element. In order for an element to have a \
             valid renderObject, it must be active, which means it is part of the tree."
        );
        self.render_object(app)
    }

    /// The size of the `RenderBox` returned by [`find_render_object`](Self::find_render_object).
    ///
    /// This getter will only return a valid result after the layout phase is complete. It is
    /// therefore not valid to call this from a build method. It should only be called from
    /// paint callbacks or interaction event handlers (e.g. gesture callbacks).
    ///
    /// This getter will only return a valid result if [`find_render_object`](Self::find_render_object)
    /// actually returns a `RenderBox`. If [`find_render_object`](Self::find_render_object)
    /// returns a render object that is not a subtype of `RenderBox` (e.g., `RenderView`),
    /// this getter will return `None`.
    pub fn size(self, app: &App) -> Option<Size> {
        debug_assert!(
            self.lifecycle(app) == ElementLifecycle::Active,
            "Cannot get size of inactive element."
        );
        debug_assert!(
            !self
                .owner(app)
                .is_some_and(|owner| owner.debug_building(app)),
            "Cannot get size during build."
        );
        let render_object = self.find_render_object(app)?;
        let render_box = render_object.as_box()?;
        debug_assert!(
            render_box.has_size(app),
            "Cannot get size from a render object that has not been through layout."
        );
        debug_assert!(
            !render_object.debug_needs_layout(app),
            "Cannot get size from a render object that has been marked dirty for layout."
        );
        Some(render_box.size(app))
    }

    /// Returns `true` if `ancestor` is a dependency of this element.
    pub fn does_depend_on_inherited_element(self, app: &App, ancestor: AnyElement) -> bool {
        self.data(app)
            .dependencies
            .as_ref()
            .is_some_and(|dependencies| dependencies.contains(&ancestor))
    }

    /// Registers this build context with `ancestor` such that when `ancestor`'s widget
    /// changes this build context is rebuilt.
    ///
    /// Returns `ancestor.widget`.
    ///
    /// This method is rarely called directly. Most applications should use
    /// [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type),
    /// which calls this method after finding the appropriate `InheritedElement` ancestor.
    pub fn depend_on_inherited_element(
        self,
        app: &mut App,
        ancestor: AnyElement,
        aspect: Option<Rc<dyn Any>>,
    ) -> WidgetRef {
        self.data_mut(app)
            .dependencies
            .get_or_insert_with(HashSet::new)
            .insert(ancestor);
        ancestor.update_dependencies(app, self, aspect);
        ancestor.widget(app).clone()
    }

    /// Returns the nearest widget of the given type `W` and creates a dependency on it, or
    /// `None` if no appropriate widget is found.
    ///
    /// The widget found will be a concrete [`InheritedWidget`] subclass, and calling this
    /// method registers this build context with the returned widget. When that widget
    /// changes (or a new widget of that type is introduced, or the widget goes away), this
    /// build context is rebuilt so that it can obtain new values from that widget.
    ///
    /// This is typically called implicitly from `of()` static methods, e.g. `Theme.of`.
    ///
    /// This method should not be called from widget constructors or from
    /// `State::init_state` methods, because those methods would not get called again if the
    /// inherited value were to change. To ensure that the widget correctly updates itself
    /// when the inherited value changes, only call this (directly or indirectly) from build
    /// methods, layout and paint callbacks, or from `State::did_change_dependencies`.
    pub fn depend_on_inherited_widget_of_exact_type<W: InheritedWidget>(
        self,
        app: &mut App,
    ) -> Option<&W> {
        self.depend_on_inherited_widget_of_exact_type_with_aspect::<W>(app, None)
    }

    /// [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type)
    /// with Dart's `aspect` argument, for `InheritedModel`.
    pub fn depend_on_inherited_widget_of_exact_type_with_aspect<W: InheritedWidget>(
        self,
        app: &mut App,
        aspect: Option<Rc<dyn Any>>,
    ) -> Option<&W> {
        debug_assert!(self.debug_check_state_is_active_for_ancestor_lookup(app));
        let ancestor = self
            .data(app)
            .inherited_elements
            .as_ref()
            .and_then(|elements| elements.get(&TypeId::of::<W>()).copied());
        let Some(ancestor) = ancestor else {
            self.data_mut(app).had_unsatisfied_dependencies = true;
            return None;
        };
        self.depend_on_inherited_element(app, ancestor, aspect);
        let widget: &WidgetRef = ancestor.widget(app);
        downcast_widget::<W>(&**widget)
    }

    /// Returns the nearest widget of the given [`InheritedWidget`] subclass `W` or `None` if
    /// an appropriate ancestor is not found.
    ///
    /// This method does not introduce a dependency the way that the more typical
    /// [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type)
    /// does, so this context will not be rebuilt if the [`InheritedWidget`] changes. This
    /// function is meant for those uncommon use cases where a dependency is undesirable.
    pub fn get_inherited_widget_of_exact_type<W: InheritedWidget>(self, app: &App) -> Option<&W> {
        let element = self.get_element_for_inherited_widget_of_exact_type::<W>(app)?;
        let widget: &WidgetRef = element.widget(app);
        downcast_widget::<W>(&**widget)
    }

    /// Obtains the element corresponding to the nearest widget of the given type `W`, which
    /// must be the type of a concrete [`InheritedWidget`] subclass.
    ///
    /// Returns `None` if no such element is found.
    ///
    /// Calling this method is O(1) with a small constant factor.
    ///
    /// This method does not establish a relationship with the target in the way that
    /// [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type)
    /// does.
    pub fn get_element_for_inherited_widget_of_exact_type<W: InheritedWidget>(
        self,
        app: &App,
    ) -> Option<AnyElement> {
        debug_assert!(self.debug_check_state_is_active_for_ancestor_lookup(app));
        self.data(app)
            .inherited_elements
            .as_ref()
            .and_then(|elements| elements.get(&TypeId::of::<W>()).copied())
    }

    fn debug_check_state_is_active_for_ancestor_lookup(self, app: &App) -> bool {
        assert!(
            self.lifecycle(app) == ElementLifecycle::Active,
            "Looking up a deactivated widget's ancestor is unsafe. At this point the state of \
             the widget's element tree is no longer stable. To safely refer to a widget's \
             ancestor in its dispose() method, save a reference to the ancestor by calling \
             dependOnInheritedWidgetOfExactType() in the widget's didChangeDependencies() \
             method."
        );
        true
    }

    /// Returns the nearest ancestor widget of the given type `W`, which must be the type of
    /// a concrete `Widget` subclass.
    ///
    /// In general, [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type)
    /// is more useful, since inherited widgets will trigger consumers to rebuild when they
    /// change. This method is appropriate when used in interaction event handlers (e.g.
    /// gesture callbacks) or for performing one-off tasks such as asserting that you have or
    /// don't have a widget of a specific type as an ancestor. The return value of a Widget's
    /// build method should not depend on the value returned by this method, because the
    /// build context will not rebuild if the return value of this method changes. This could
    /// lead to a situation where data used in the build method changes, but the widget is
    /// not rebuilt.
    ///
    /// Calling this method is relatively expensive (O(N) in the depth of the tree). Only
    /// call this method if the distance from this widget to the desired ancestor is known to
    /// be small and bounded.
    pub fn find_ancestor_widget_of_exact_type<W: 'static>(self, app: &App) -> Option<&W> {
        debug_assert!(self.debug_check_state_is_active_for_ancestor_lookup(app));
        let mut ancestor = self.parent(app);
        while let Some(current) = ancestor {
            if current.widget(app).widget_type() == TypeId::of::<W>() {
                return downcast_widget::<W>(&**current.widget(app));
            }
            ancestor = current.parent(app);
        }
        None
    }

    /// Returns the [`State`] object of the nearest ancestor `StatefulWidget` widget that is
    /// an instance of the given type `S`.
    ///
    /// This should not be used from build methods, because the build context will not be
    /// rebuilt if the value that would be returned by this method changes. In general,
    /// [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type)
    /// is more appropriate for such cases. This method is useful for changing the state of an
    /// ancestor widget in a one-off manner, for example, to cause an ancestor scrolling list
    /// to scroll this build context's widget into view, or to move the focus in response to
    /// user interaction.
    ///
    /// Calling this method is relatively expensive (O(N) in the depth of the tree). Only
    /// call this method if the distance from this widget to the desired ancestor is known to
    /// be small and bounded.
    pub fn find_ancestor_state_of_type<S: State>(self, app: &App) -> Option<Handle<S>> {
        debug_assert!(self.debug_check_state_is_active_for_ancestor_lookup(app));
        let mut ancestor = self.parent(app);
        while let Some(current) = ancestor {
            if let Some(state) = current.state_handle::<S>(app) {
                return Some(state);
            }
            ancestor = current.parent(app);
        }
        None
    }

    /// Returns the [`State`] object of the furthest ancestor `StatefulWidget` widget that is
    /// an instance of the given type `S`.
    ///
    /// Functions the same way as [`find_ancestor_state_of_type`](Self::find_ancestor_state_of_type)
    /// but keeps visiting subsequent ancestors until there are none of the type instance of
    /// `S` remaining. Then returns the last one found.
    ///
    /// This operation is O(N) as well though N is the entire widget tree rather than a
    /// subtree.
    pub fn find_root_ancestor_state_of_type<S: State>(self, app: &App) -> Option<Handle<S>> {
        debug_assert!(self.debug_check_state_is_active_for_ancestor_lookup(app));
        let mut ancestor = self.parent(app);
        let mut stateful_ancestor = None;
        while let Some(current) = ancestor {
            if let Some(state) = current.state_handle::<S>(app) {
                stateful_ancestor = Some(state);
            }
            ancestor = current.parent(app);
        }
        stateful_ancestor
    }

    /// Returns the `RenderObject` object of the nearest ancestor `RenderObjectWidget` widget
    /// that is an instance of the given type `R`.
    ///
    /// This should not be used from build methods, because the build context will not be
    /// rebuilt if the value that would be returned by this method changes. In general,
    /// [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type)
    /// is more appropriate for such cases. This method is useful only in esoteric cases where
    /// a widget needs to cause an ancestor to change its layout or paint behavior. For
    /// example, it is used by `Material` so that `InkWell` widgets can trigger the ink splash
    /// on the `Material`'s actual render object.
    ///
    /// Calling this method is relatively expensive (O(N) in the depth of the tree). Only
    /// call this method if the distance from this widget to the desired ancestor is known to
    /// be small and bounded.
    pub fn find_ancestor_render_object_of_type<R: RenderObject>(
        self,
        app: &App,
    ) -> Option<RenderHandle<R>> {
        debug_assert!(self.debug_check_state_is_active_for_ancestor_lookup(app));
        let mut ancestor = self.parent(app);
        while let Some(current) = ancestor {
            if current.is_render_object_element()
                && let Some(render_object) = current.render_object(app)
                && let Some(typed) = render_object.downcast::<R>(app)
            {
                return Some(typed);
            }
            ancestor = current.parent(app);
        }
        None
    }

    /// Walks the ancestor chain, starting with the parent of this build context's widget,
    /// invoking the argument for each ancestor.
    ///
    /// The callback is given a reference to the ancestor widget's corresponding [`Element`]
    /// object. The walk stops when it reaches the root widget or when the callback returns
    /// false. The callback must not return `None`.
    ///
    /// This is useful for inspecting the widget tree.
    ///
    /// Calling this method is relatively expensive (O(N) in the depth of the tree).
    pub fn visit_ancestor_elements(self, app: &App, visitor: &mut dyn FnMut(AnyElement) -> bool) {
        debug_assert!(self.debug_check_state_is_active_for_ancestor_lookup(app));
        let mut ancestor = self.parent(app);
        while let Some(current) = ancestor {
            if !visitor(current) {
                break;
            }
            ancestor = current.parent(app);
        }
    }

    /// Walks the children of this widget.
    ///
    /// This is useful for applying changes to children after they are built without waiting
    /// for the next frame, especially if the children are known, and especially if there is
    /// exactly one child (as is always the case for `StatefulWidget`s or `StatelessWidget`s).
    ///
    /// Calling this method is very cheap for build contexts that correspond to
    /// `StatefulWidget`s or `StatelessWidget`s (O(1), since there's only one child).
    ///
    /// Calling this method is potentially expensive for build contexts that correspond to
    /// `RenderObjectWidget`s (O(N) in the number of children).
    ///
    /// Calling this method recursively is extremely expensive (O(N) in the number of
    /// descendants), and should be avoided if possible. Generally it is significantly cheaper
    /// to use an `InheritedWidget` and have the descendants pull data down than it is to use
    /// [`visit_child_elements`](Self::visit_child_elements) recursively to push data down to
    /// them.
    pub fn visit_child_elements(self, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        debug_assert!(
            !self
                .owner(app)
                .is_some_and(|owner| owner.debug_state_locked(app)),
            "visitChildElements() called during build."
        );
        self.visit_children(app, visitor);
    }

    /// Dart's `_debugIsDescendantOf`.
    pub fn debug_is_descendant_of(self, app: &App, target: AnyElement) -> bool {
        let mut element = Some(self);
        while let Some(current) = element
            && current.depth(app) > target.depth(app)
        {
            element = current.parent(app);
        }
        element == Some(target)
    }

    /// Marks the element as dirty and adds it to the global list of widgets to rebuild in the
    /// next frame.
    ///
    /// Since it is inefficient to build an element twice in one frame, applications and
    /// widgets should be structured so as to only mark widgets dirty during event handlers
    /// before the frame begins, not during the build itself.
    pub fn mark_needs_build(self, app: &mut App) {
        (self.vtable.mark_needs_build)(app, self.id);
    }

    /// Cause the widget to update itself. In debug builds, also verify various invariants.
    ///
    /// Called by the [`BuildOwner`] when `BuildOwner::schedule_build_for` has been called to
    /// mark this element dirty, by [`mount`](Self::mount) when the element is first built,
    /// and by [`update`](Self::update) when the widget has changed.
    ///
    /// The method will only rebuild if [`dirty`](Self::dirty) is true. To rebuild
    /// irrespective of the [`dirty`](Self::dirty) flag, set `force` to true. Forcing a
    /// rebuild is convenient when [`update`](Self::update) is called, during which
    /// [`dirty`](Self::dirty) is false.
    pub fn rebuild(self, app: &mut App, force: bool) {
        debug_assert!(self.lifecycle(app) != ElementLifecycle::Initial);
        if self.lifecycle(app) != ElementLifecycle::Active || (!self.dirty(app) && !force) {
            return;
        }
        let owner = self.owner(app).expect("an active element has an owner");
        debug_assert!(owner.debug_state_locked(app));
        let debug_previous_build_target = owner.debug_current_build_target(app);
        if cfg!(debug_assertions) {
            owner.set_debug_current_build_target(app, Some(self));
            self.data_mut(app).debug_built_once = true;
        }
        self.perform_rebuild(app);
        if cfg!(debug_assertions) {
            debug_assert!(owner.debug_current_build_target(app) == Some(self));
            owner.set_debug_current_build_target(app, debug_previous_build_target);
        }
        debug_assert!(!self.dirty(app));
    }

    pub(crate) fn had_dependencies(self, app: &App) -> bool {
        let data = self.data(app);
        data.dependencies
            .as_ref()
            .is_some_and(|dependencies| !dependencies.is_empty())
            || data.had_unsatisfied_dependencies
    }

    /// The notification chain this element dispatches into.
    pub fn notification_tree(self, app: &App) -> Option<Rc<NotificationNode>> {
        self.data(app).notification_tree.clone()
    }

    /// Sets the notification chain: a notifiable element chains a node for itself here.
    pub fn set_notification_tree(self, app: &mut App, tree: Option<Rc<NotificationNode>>) {
        self.data_mut(app).notification_tree = tree;
    }

    pub(crate) fn inherited_elements(self, app: &App) -> Option<Rc<HashMap<TypeId, AnyElement>>> {
        self.data(app).inherited_elements.clone()
    }

    pub(crate) fn set_inherited_elements(
        self,
        app: &mut App,
        elements: Option<Rc<HashMap<TypeId, AnyElement>>>,
    ) {
        self.data_mut(app).inherited_elements = elements;
    }

    pub(crate) fn set_slot(self, app: &mut App, slot: Option<Slot>) {
        self.data_mut(app).slot = slot;
    }

    pub(crate) fn set_dirty(self, app: &mut App, dirty: bool) {
        self.data_mut(app).dirty = dirty;
    }

    /// `RootElementMixin.assignOwner`: set the owner of the element and its build scope.
    ///
    /// The owner of an element is determined at mount, so this should only be called on a
    /// root element that is about to be mounted without a parent.
    pub fn assign_owner(self, app: &mut App, owner: Handle<BuildOwner>) {
        let scope = BuildScope::new(app, None);
        let data = self.data_mut(app);
        data.owner = Some(owner);
        data.parent_build_scope = Some(scope);
    }
}

/// A widget key compared as Dart's `Map<Key, Element>` compares it.
struct KeyIdentity(super::widget::KeyRef);

impl PartialEq for KeyIdentity {
    fn eq(&self, other: &KeyIdentity) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for KeyIdentity {}

impl Hash for KeyIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self.0).hash(state);
    }
}

fn activate_recursively(app: &mut App, element: AnyElement) {
    debug_assert!(element.lifecycle(app) == ElementLifecycle::Inactive);
    element.activate(app);
    debug_assert!(element.lifecycle(app) == ElementLifecycle::Active);
    for child in element.children(app) {
        activate_recursively(app, child);
    }
}

// ---------------------------------------------------------------------------------------------
// Dart's `Element` base bodies. An override calls the one it would call as `super`.

/// Dart's `Element` bodies, reachable from an override the way `super.mount(..)` is.
///
/// Every [`Element`] implements it. An override that must run the inherited body calls it
/// trait-qualified, `ElementBase::mount(self, app, parent, new_slot)`, at the position Dart
/// calls `super`; the [`Element`] defaults run the same bodies.
pub trait ElementBase: Element {
    /// `Element.renderObjectAttachingChild`: the single child, if any.
    fn render_object_attaching_child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        let this = self.as_element();
        let mut next = None;
        this.visit_children(app, &mut |child| {
            debug_assert!(next.is_none()); // This verifies that there's only one child.
            next = Some(child);
        });
        next
    }

    /// `Element.mount`.
    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        let this = self.as_element();
        debug_assert!(
            this.lifecycle(app) == ElementLifecycle::Initial,
            "This element is no longer in its initial state ({:?})",
            this.lifecycle(app)
        );
        debug_assert!(
            this.parent(app).is_none(),
            "This element already has a parent and it shouldn't have one yet."
        );
        debug_assert!(
            parent.is_none_or(|parent| parent.lifecycle(app) == ElementLifecycle::Active),
            "Parent should be null or in the active state"
        );
        debug_assert!(
            this.slot(app).is_none(),
            "This element already has a slot and it shouldn't"
        );
        let parent_depth = parent.map_or(0, |parent| parent.depth(app));
        let parent_owner = parent.and_then(|parent| parent.owner(app));
        let parent_scope = parent.map(|parent| parent.build_scope(app));
        let data = this.data_mut(app);
        data.parent = parent;
        data.slot = new_slot;
        data.lifecycle = ElementLifecycle::Active;
        data.depth = 1 + parent_depth;
        if parent.is_some() {
            // Only assign ownership if the parent is non-null. If parent is null
            // (the root node), the owner should have already been assigned.
            // See RootRenderObjectElement.assignOwner().
            data.owner = parent_owner;
            data.parent_build_scope = parent_scope;
        }
        let owner = this.owner(app).expect("a mounted element has an owner");
        if let Some(key) = this.widget(app).key().and_then(global_key_id) {
            owner.register_global_key(app, key, this);
        }
        this.update_inheritance(app);
        this.attach_notification_tree(app);
    }

    /// `Element.update`.
    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let this = self.as_element();
        // This code is hot when hot reloading, so we try to
        // only call _AssertionError._evaluateAssertion once.
        debug_assert!(
            this.lifecycle(app) == ElementLifecycle::Active
                && !same_widget(this.widget(app), &new_widget)
                && can_update(&**this.widget(app), &*new_widget)
        );
        this.data_mut(app).widget = Some(new_widget);
    }

    /// `Element.updateSlot`.
    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        let this = self.as_element();
        debug_assert!(this.lifecycle(app) == ElementLifecycle::Active);
        debug_assert!(this.parent(app).is_some());
        debug_assert!(
            this.parent(app)
                .is_some_and(|parent| parent.lifecycle(app) == ElementLifecycle::Active)
        );
        this.set_slot(app, new_slot);
    }

    /// `Element.attachRenderObject`.
    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        let this = self.as_element();
        debug_assert!(this.slot(app).is_none());
        for child in this.children(app) {
            child.attach_render_object(app, new_slot.clone());
        }
        this.set_slot(app, new_slot);
    }

    /// `Element.detachRenderObject`.
    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        for child in this.children(app) {
            child.detach_render_object(app);
        }
        this.set_slot(app, None);
    }

    /// `Element.activate`.
    fn activate(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        debug_assert!(this.lifecycle(app) == ElementLifecycle::Inactive);
        let owner = this
            .owner(app)
            .expect("an inactive element keeps its owner");
        let had_dependencies = this.had_dependencies(app);
        let data = this.data_mut(app);
        data.lifecycle = ElementLifecycle::Active;
        // We unregistered our dependencies in deactivate, but never cleared the list.
        // Since we're going to be reused, let's clear our list now.
        if let Some(dependencies) = &mut data.dependencies {
            dependencies.clear();
        }
        data.had_unsatisfied_dependencies = false;
        this.update_inheritance(app);
        this.attach_notification_tree(app);
        if this.dirty(app) {
            owner.schedule_build_for(app, this);
        }
        if had_dependencies {
            this.did_change_dependencies(app);
        }
    }

    /// `Element.deactivate`.
    fn deactivate(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        debug_assert!(this.lifecycle(app) == ElementLifecycle::Active);
        debug_assert!(this.mounted(app));
        this.ensure_deactivated(app);
    }

    /// `Element.unmount`.
    fn unmount(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        debug_assert!(this.lifecycle(app) == ElementLifecycle::Inactive);
        debug_assert!(this.mounted(app));
        let owner = this
            .owner(app)
            .expect("an inactive element keeps its owner");
        if let Some(key) = this.widget(app).key().and_then(global_key_id) {
            owner.unregister_global_key(app, key, this);
        }
        // Release resources to reduce the severity of memory leaks caused by
        // defunct, but accidentally retained Elements.
        let data = this.data_mut(app);
        data.widget = None;
        data.dependencies = None;
        data.lifecycle = ElementLifecycle::Defunct;
    }

    /// `Element.performRebuild`: clears the dirty flag.
    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        this.set_dirty(app, false);
    }

    /// `Element.didChangeDependencies`.
    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        debug_assert!(this.lifecycle(app) == ElementLifecycle::Active); // otherwise markNeedsBuild is a no-op
        debug_assert!(debug_check_owner_build_target_exists(
            this,
            app,
            "didChangeDependencies"
        ));
        this.mark_needs_build(app);
    }

    /// `Element.attachNotificationTree`: inherit the parent's notification chain.
    fn attach_notification_tree(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        let tree = this
            .parent(app)
            .and_then(|parent| parent.data(app).notification_tree.clone());
        this.data_mut(app).notification_tree = tree;
    }

    /// `Element.buildScope`: the scope inherited from the parent at mount.
    fn build_scope(self: Handle<Self>, app: &App) -> Handle<BuildScope> {
        self.as_element()
            .data(app)
            .parent_build_scope
            .expect("an element in a tree has a build scope")
    }

    /// `Element.markNeedsBuild`.
    fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        debug_assert!(this.lifecycle(app) != ElementLifecycle::Defunct);
        if this.lifecycle(app) != ElementLifecycle::Active {
            return;
        }
        let owner = this.owner(app).expect("an active element has an owner");
        if cfg!(debug_assertions) {
            if owner.debug_building(app) {
                let target = owner
                    .debug_current_build_target(app)
                    .expect("building has a target");
                debug_assert!(owner.debug_state_locked(app));
                assert!(
                    this.debug_is_descendant_of(app, target),
                    "setState() or markNeedsBuild() called during build. This widget cannot \
                     be marked as needing to build because the framework is already in the \
                     process of building widgets. A widget can be marked as needing to be \
                     built during the build phase only if one of its ancestors is currently \
                     building."
                );
            } else {
                assert!(
                    !owner.debug_state_locked(app),
                    "setState() or markNeedsBuild() called when widget tree was locked."
                );
            }
        }
        if this.dirty(app) {
            return;
        }
        this.data_mut(app).dirty = true;
        owner.schedule_build_for(app, this);
    }

    /// `Element._updateInheritance`.
    fn update_inheritance(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        debug_assert!(this.lifecycle(app) == ElementLifecycle::Active);
        let inherited = this
            .parent(app)
            .and_then(|parent| parent.inherited_elements(app));
        this.set_inherited_elements(app, inherited);
    }

    /// `Element.renderObject`: walk down through the attaching children.
    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        let this = self.as_element();
        let mut current = Some(this);
        while let Some(element) = current {
            if element.lifecycle(app) == ElementLifecycle::Defunct {
                break;
            } else if element.is_render_object_element() {
                return element.render_object(app);
            } else {
                current = element.render_object_attaching_child(app);
            }
        }
        None
    }

    /// `Element.inflateWidget`.
    fn inflate_widget(
        self: Handle<Self>,
        app: &mut App,
        new_widget: WidgetRef,
        new_slot: Option<Slot>,
    ) -> AnyElement {
        let this = self.as_element();
        let inactive_child = new_widget
            .key()
            .and_then(global_key_id)
            .and_then(|key| this.retake_inactive_element(app, key, &new_widget));
        match inactive_child {
            Some(inactive_child) => {
                debug_assert!(inactive_child.parent(app).is_none());
                inactive_child.activate_with_parent(app, this, new_slot.clone());
                let updated_child =
                    this.update_child(app, Some(inactive_child), Some(new_widget), new_slot);
                debug_assert!(updated_child == Some(inactive_child));
                updated_child.expect("a retaken child stays")
            }
            None => {
                let new_child = new_widget.create_element(app, new_widget.clone());
                debug_assert!(debug_check_for_cycles(this, app, new_child));
                new_child.mount(app, Some(this), new_slot);
                debug_assert!(new_child.lifecycle(app) == ElementLifecycle::Active);
                new_child
            }
        }
    }
}
impl<T: Element> ElementBase for T {}

/// Dart's `_debugCheckOwnerBuildTargetExists`.
pub(crate) fn debug_check_owner_build_target_exists(
    this: AnyElement,
    app: &App,
    method_name: &str,
) -> bool {
    let owner = this.owner(app).expect("an active element has an owner");
    assert!(
        owner.debug_current_build_target(app).is_some(),
        "{method_name} for {:?} was called at an inappropriate time. It may only be called \
         while the widgets are being built.",
        this.widget(app)
    );
    true
}

fn debug_check_for_cycles(this: AnyElement, app: &App, new_child: AnyElement) -> bool {
    debug_assert!(new_child.parent(app).is_none());
    let mut node = this;
    while let Some(parent) = node.parent(app) {
        node = parent;
    }
    assert!(
        node != new_child,
        "about to create a cycle in the element tree"
    );
    true
}

/// The widget kinds the superclass check compares.
pub fn widget_kind(widget: &dyn Widget) -> WidgetKind {
    widget.kind()
}

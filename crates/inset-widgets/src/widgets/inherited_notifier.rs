//! Flutter counterpart: `widgets/inherited_notifier.dart`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;

use inset_foundation::{App, Handle, ListenableObject, Listener};

use crate::framework::{
    AnyElement, ComponentElement, ComponentElementData, Element, ElementBase, ElementData,
    ElementLifecycle, InheritedWidget, IntoWidget, KeyRef, ProxyElement, Slot, Widget, WidgetKind,
    WidgetRef, debug_check_owner_build_target_exists, downcast_widget,
};

/// An inherited widget for a [`Listenable`](inset_foundation::Listenable) [`notifier`](Self::notifier), which updates its
/// dependencies when the notifier is triggered.
///
/// This is a variant of [`InheritedWidget`], specialized for implementors of
/// [`ListenableObject`], such as `ChangeNotifier` or `ValueNotifier`.
///
/// Dependents are notified whenever the notifier sends notifications, or
/// whenever the identity of the notifier changes.
///
/// Multiple notifications are coalesced, so that dependents only rebuild once
/// even if the notifier fires multiple times between two frames.
///
/// Typically this trait is implemented by a widget that provides an `of` associated function
/// calling [`BuildContext::depend_on_inherited_widget_of_exact_type`](crate::AnyElement::depend_on_inherited_widget_of_exact_type)
/// with that widget's type. The widget's `impl InheritedWidget` writes
/// [`inherited_notifier_overrides!`](crate::inherited_notifier_overrides) to take Dart's
/// `updateShouldNotify`, which compares the two notifiers; an implementor that wants other
/// logic writes its own `update_should_notify` instead.
///
/// A widget is both an [`InheritedWidget`] and an [`InheritedNotifier`], so the kind tag of
/// [`IntoWidget`] cannot be inferred for it: the widget type names the conversion itself
/// (`fn into_widget(self) -> WidgetRef { IntoWidget::<InheritedNotifierKind>::into_widget(self) }`).
///
/// See also:
///
///  * `Animation`, an implementation of [`Listenable`](inset_foundation::Listenable) that ticks each frame to
///    update a value.
///  * [`ViewportOffset`](inset_rendering::ViewportOffset) or `ScrollPosition`,
///    implementations of [`Listenable`](inset_foundation::Listenable) that trigger when a view is scrolled.
///  * [`InheritedWidget`], an inherited widget that only notifies dependents
///    when its value is different.
///  * [`InheritedModel`](crate::InheritedModel), an inherited widget that allows clients to
///    subscribe to changes for subparts of the value.
pub trait InheritedNotifier: InheritedWidget {
    /// The type of the notifier; Dart's type parameter `T extends Listenable`.
    type Notifier: ListenableObject;

    /// The [`Listenable`](inset_foundation::Listenable) object to which to listen.
    ///
    /// Whenever this object sends change notifications, the dependents of this
    /// widget are triggered.
    ///
    /// By default, whenever the notifier is changed (including when changing to
    /// or from `None`), if the old notifier is not equal to the new notifier,
    /// notifications are sent. This behavior can be overridden by writing
    /// [`InheritedWidget::update_should_notify`] by hand instead of using
    /// [`inherited_notifier_overrides!`](crate::inherited_notifier_overrides).
    ///
    /// While the notifier is `None`, no notifications are sent, since the missing
    /// object cannot itself send notifications.
    fn notifier(&self) -> Option<Handle<Self::Notifier>>;
}

/// Dart's `InheritedNotifier.updateShouldNotify`. Write it inside
/// `impl InheritedWidget for MyWidget { .. }`.
#[macro_export]
macro_rules! inherited_notifier_overrides {
    () => {
        fn update_should_notify(&self, old_widget: &Self) -> bool {
            $crate::InheritedNotifier::notifier(old_widget)
                != $crate::InheritedNotifier::notifier(self)
        }
    };
}

/// The kind tag of [`IntoWidget`] for an [`InheritedNotifier`].
pub struct InheritedNotifierKind;

/// The erased form of an [`InheritedNotifier`].
pub struct InheritedNotifierWrapper<W: InheritedNotifier>(pub W);

impl<W: InheritedNotifier> IntoWidget<InheritedNotifierKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(InheritedNotifierWrapper(self))
    }
}

impl<W: InheritedNotifier> Widget for InheritedNotifierWrapper<W> {
    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        InheritedNotifierElement::<W>::create(app, this)
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: InheritedNotifier> Debug for InheritedNotifierWrapper<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------------------------------------
// InheritedNotifierElement

/// An [`Element`] that uses an [`InheritedNotifier`] as its configuration; Dart's
/// `_InheritedNotifierElement`.
pub struct InheritedNotifierElement<W: InheritedNotifier> {
    element: ElementData,
    component: ComponentElementData,
    dependents: HashMap<AnyElement, Option<Rc<dyn Any>>>,
    dirty: bool,
    marker: PhantomData<fn() -> W>,
}

impl<W: InheritedNotifier> InheritedNotifierElement<W> {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        debug_assert!(downcast_widget::<W>(&*widget).is_some());
        let this = app.create(InheritedNotifierElement::<W> {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            dependents: HashMap::new(),
            dirty: false,
            marker: PhantomData,
        });
        if let Some(notifier) = this.widget(app).notifier() {
            notifier.add_listener(app, this.update_listener());
        }
        this.as_element()
    }

    /// The widget, typed.
    pub fn widget(self: Handle<Self>, app: &App) -> &W {
        downcast_widget::<W>(&**self.as_element().widget(app))
            .expect("an InheritedNotifierElement holds its InheritedNotifier")
    }

    /// Dart's `_handleUpdate` tear-off, which the notifier is listened to with.
    fn update_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, InheritedNotifierElement::<W>::handle_update)
    }

    /// Dart's `_handleUpdate`.
    fn handle_update(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).dirty = true;
        self.as_element().mark_needs_build(app);
    }

    /// See [`InheritedElement::get_dependencies`](crate::InheritedElement::get_dependencies).
    pub fn get_dependencies(
        self: Handle<Self>,
        app: &App,
        dependent: AnyElement,
    ) -> Option<Rc<dyn Any>> {
        app.get(self).dependents.get(&dependent).cloned().flatten()
    }

    /// See [`InheritedElement::set_dependencies`](crate::InheritedElement::set_dependencies).
    pub fn set_dependencies(
        self: Handle<Self>,
        app: &mut App,
        dependent: AnyElement,
        value: Option<Rc<dyn Any>>,
    ) {
        app.get_mut(self).dependents.insert(dependent, value);
    }

    /// See [`InheritedElement::notify_dependent`](crate::InheritedElement::notify_dependent).
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

impl<W: InheritedNotifier> ComponentElement for InheritedNotifierElement<W> {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        if app.get(self).dirty {
            let widget = self.as_element().widget(app).clone();
            self.notify_clients(app, widget);
        }
        self.proxied_child(app)
    }
}

impl<W: InheritedNotifier> ProxyElement for InheritedNotifierElement<W> {
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
        app.get_mut(self).dirty = false;
    }
}

impl<W: InheritedNotifier> Element for InheritedNotifierElement<W> {
    crate::element_accessors!();

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

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_notifier = self.widget(app).notifier();
        let new_notifier = downcast_widget::<W>(&*new_widget)
            .expect("the new widget has the same type")
            .notifier();
        if old_notifier != new_notifier {
            if let Some(old_notifier) = old_notifier {
                old_notifier.remove_listener(app, &self.update_listener());
            }
            if let Some(new_notifier) = new_notifier {
                new_notifier.add_listener(app, self.update_listener());
            }
        }
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

    fn unmount(self: Handle<Self>, app: &mut App) {
        if let Some(notifier) = self.widget(app).notifier() {
            notifier.remove_listener(app, &self.update_listener());
        }
        ElementBase::unmount(self, app);
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

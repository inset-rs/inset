//! Flutter counterpart: `widgets/notification_listener.dart`.

use std::any::{Any, TypeId};
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

use inset_foundation::{App, Handle};

use crate::framework::{
    AnyElement, ComponentElement, ComponentElementData, Element, ElementData, IntoWidget, KeyRef,
    Notification, NotificationNode, NotificationTarget, ProxyElement, Slot, Widget, WidgetKind,
    WidgetRef, component_element_overrides, downcast_widget,
};

/// Signature for [`NotificationListener::on_notification`].
///
/// Return true to cancel the notification bubbling. Return false to allow the notification
/// to continue to be dispatched to further ancestors.
///
/// Used by [`NotificationListener::on_notification`].
pub type NotificationListenerCallback<T> = Rc<dyn Fn(&mut App, &T) -> bool>;

/// A widget that listens for [`Notification`]s bubbling up the tree.
///
/// Notifications will trigger the [`on_notification`](Self::on_notification) callback only if
/// their `runtimeType` is a subtype of `T`: a concrete notification type, or a family such
/// as `dyn ScrollNotification` (see [`NotificationTarget`]).
///
/// To dispatch notifications, use the [`Notification::dispatch`] method.
pub struct NotificationListener<T: ?Sized + NotificationTarget> {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
    /// Called when a notification of the appropriate type arrives at this location in the
    /// tree.
    ///
    /// Return true to cancel the notification bubbling. Return false to allow the
    /// notification to continue to be dispatched to further ancestors.
    ///
    /// Notifications vary in terms of when they are dispatched. There are two main
    /// possibilities: dispatch between frames, and dispatch during layout.
    ///
    /// For notifications that dispatch during layout, such as those that inherit from
    /// `LayoutChangedNotification`, it is too late to call `State::set_state` in response
    /// to the notification (as layout is currently happening in a descendant, by definition,
    /// since notifications bubble up the tree). For widgets that depend on layout, consider
    /// a `LayoutBuilder` instead.
    pub on_notification: Option<NotificationListenerCallback<T>>,
}

impl<T: ?Sized + NotificationTarget> NotificationListener<T> {
    /// Creates a widget that listens for notifications.
    pub fn new<K>(child: impl IntoWidget<K>) -> NotificationListener<T> {
        NotificationListener {
            key: None,
            child: child.into_widget(),
            on_notification: None,
        }
    }

    /// Dart `NotificationListener(key:)`.
    pub fn key(mut self, key: KeyRef) -> NotificationListener<T> {
        self.key = Some(key);
        self
    }

    /// Dart `NotificationListener(onNotification:)`.
    pub fn on_notification(
        mut self,
        on_notification: impl Fn(&mut App, &T) -> bool + 'static,
    ) -> NotificationListener<T> {
        self.on_notification = Some(Rc::new(on_notification));
        self
    }

    /// The tree node: a `ProxyWidget` with its own element, outside the `IntoWidget` kinds.
    pub fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }
}

impl<T: ?Sized + NotificationTarget> fmt::Debug for NotificationListener<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotificationListener")
            .field("notification", &std::any::type_name::<T>())
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl<T: ?Sized + NotificationTarget> Widget for NotificationListener<T> {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        NotificationElement::<T>::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<NotificationListener<T>>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

/// Dart's `_NotificationElement<T>`: a proxy element that is a node in the notification tree.
pub struct NotificationElement<T: ?Sized + NotificationTarget> {
    element: ElementData,
    component: ComponentElementData,
    marker: PhantomData<fn() -> T>,
}

impl<T: ?Sized + NotificationTarget> NotificationElement<T> {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<NotificationElement<T>> {
        debug_assert!(downcast_widget::<NotificationListener<T>>(&*widget).is_some());
        app.create(NotificationElement {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            marker: PhantomData,
        })
    }

    fn widget(self: Handle<Self>, app: &App) -> &NotificationListener<T> {
        downcast_widget::<NotificationListener<T>>(&**self.as_element().widget(app))
            .expect("a NotificationElement holds its NotificationListener")
    }
}

impl<T: ?Sized + NotificationTarget> ComponentElement for NotificationElement<T> {
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

impl<T: ?Sized + NotificationTarget> ProxyElement for NotificationElement<T> {
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
        self.widget(app).child.clone()
    }

    fn notify_clients(self: Handle<Self>, _app: &mut App, _old_widget: WidgetRef) {
        // Notification tree does not need to notify clients.
    }
}

impl<T: ?Sized + NotificationTarget> Element for NotificationElement<T> {
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

    /// `NotifiableElementMixin.attachNotificationTree`: a node for this element.
    fn attach_notification_tree(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        let parent_tree = this
            .parent(app)
            .and_then(|parent| parent.notification_tree(app));
        this.set_notification_tree(app, Some(Rc::new(NotificationNode::new(parent_tree, this))));
    }

    fn on_notification(self: Handle<Self>, app: &mut App, notification: &dyn Notification) -> bool {
        let Some(on_notification) = self.widget(app).on_notification.clone() else {
            return false;
        };
        match T::cast(notification) {
            Some(notification) => on_notification(app, notification),
            None => false,
        }
    }
}

/// Indicates that the layout of one of the descendants of the object receiving this
/// notification has changed in some way, and that therefore any assumptions about that
/// layout are no longer valid.
///
/// Useful if, for instance, you're trying to align multiple descendants.
///
/// To listen for notifications in a subtree, use a [`NotificationListener`].
#[derive(Debug, Default)]
pub struct LayoutChangedNotification;

impl Notification for LayoutChangedNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};

    #[derive(Debug)]
    struct Ping(u32);

    impl Notification for Ping {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct Other;

    impl Notification for Other {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    type Log = Rc<RefCell<Vec<&'static str>>>;

    /// A leaf that dispatches `Ping(7)` and `Other` from its own context when it builds.
    fn dispatcher() -> WidgetRef {
        Builder::new(|app, context| {
            Ping(7).dispatch(app, Some(context));
            Other.dispatch(app, Some(context));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    fn listener(log: &Log, name: &'static str, cancel: bool, child: WidgetRef) -> WidgetRef {
        let log = Rc::clone(log);
        NotificationListener::<Ping>::new(child)
            .on_notification(move |_app, ping| {
                assert_eq!(ping.0, 7);
                log.borrow_mut().push(name);
                cancel
            })
            .into_widget()
    }

    #[test]
    fn a_notification_bubbles_through_listeners_of_its_type_until_one_cancels() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log: Log = Rc::default();
        let tree = listener(
            &log,
            "outer",
            false,
            listener(
                &log,
                "middle",
                true,
                listener(&log, "inner", false, dispatcher()),
            ),
        );
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);
        assert_eq!(*log.borrow(), vec!["inner", "middle"]);
    }

    #[test]
    fn a_listener_of_another_type_is_skipped_and_dispatching_without_a_tree_is_a_no_op() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log: Log = Rc::default();
        let other_log: Log = Rc::default();
        let other = {
            let other_log = Rc::clone(&other_log);
            NotificationListener::<Other>::new(dispatcher())
                .on_notification(move |_app, _other| {
                    other_log.borrow_mut().push("other");
                    true
                })
                .into_widget()
        };
        let harness = Harness::mount(&mut app, listener(&log, "outer", false, other));
        harness.pump(&mut app);
        assert_eq!(*log.borrow(), vec!["outer"]);
        assert_eq!(*other_log.borrow(), vec!["other"]);

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, dispatcher());
        harness.pump(&mut app);
    }
}

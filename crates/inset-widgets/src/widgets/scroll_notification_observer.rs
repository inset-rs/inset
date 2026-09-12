//! Flutter counterpart: `widgets/scroll_notification_observer.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_foundation::{App, Handle};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::scroll_notification::ScrollNotification;
use crate::widgets::scroll_position::ScrollMetricsNotification;

/// A [`ScrollNotification`] listener for [`ScrollNotificationObserver`].
///
/// [`ScrollNotificationObserver`] is similar to
/// [`NotificationListener`]. It supports a listener list instead of
/// just a single listener and its listeners run unconditionally, they
/// do not require a gating boolean return value.
pub type ScrollNotificationCallback = Rc<dyn Fn(&mut App, &dyn ScrollNotification)>;

/// Dart's `_ScrollNotificationObserverScope`.
#[derive(Debug)]
struct ScrollNotificationObserverScope {
    scroll_notification_observer_state: Handle<ScrollNotificationObserverState>,
    child: WidgetRef,
}

impl InheritedWidget for ScrollNotificationObserverScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &ScrollNotificationObserverScope) -> bool {
        self.scroll_notification_observer_state != old.scroll_notification_observer_state
    }
}

/// Notifies its listeners when a descendant scrolls.
///
/// To add a listener to a [`ScrollNotificationObserver`] ancestor:
///
/// ```text
/// ScrollNotificationObserver::of(app, context).add_listener(app, listener);
/// ```
///
/// To remove the listener from a [`ScrollNotificationObserver`] ancestor:
///
/// ```text
/// ScrollNotificationObserver::of(app, context).remove_listener(app, &listener);
/// ```
///
/// Stateful widgets that share an ancestor [`ScrollNotificationObserver`] typically
/// add a listener in [`State::did_change_dependencies`] (removing the old one
/// if necessary) and remove the listener in their [`State::dispose`] method.
///
/// This widget is similar to [`NotificationListener`]. It supports a listener
/// list instead of just a single listener and its listeners run
/// unconditionally, they do not require a gating boolean return value.
#[derive(Debug)]
pub struct ScrollNotificationObserver {
    pub key: Option<KeyRef>,

    /// The subtree below this widget.
    pub child: WidgetRef,
}

impl ScrollNotificationObserver {
    /// Create a [`ScrollNotificationObserver`].
    pub fn new<K>(child: impl IntoWidget<K>) -> ScrollNotificationObserver {
        ScrollNotificationObserver {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `ScrollNotificationObserver(key:)`.
    pub fn key(mut self, key: KeyRef) -> ScrollNotificationObserver {
        self.key = Some(key);
        self
    }

    /// The closest instance of this class that encloses the given context.
    ///
    /// If there is no enclosing [`ScrollNotificationObserver`] widget, then `None` is
    /// returned.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`ScrollNotificationObserver`] in the `context`, if there is one.
    ///
    /// See also:
    ///
    /// * [`ScrollNotificationObserver::of`], which is similar to this method, but
    ///   panics if no [`ScrollNotificationObserver`] ancestor is found.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<Handle<ScrollNotificationObserverState>> {
        context
            .depend_on_inherited_widget_of_exact_type::<ScrollNotificationObserverScope>(app)
            .map(|scope| scope.scroll_notification_observer_state)
    }

    /// The closest instance of this class that encloses the given context.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`ScrollNotificationObserver`] in the `context`.
    ///
    /// # Panics
    ///
    /// If no [`ScrollNotificationObserver`] ancestor is found.
    ///
    /// See also:
    ///
    /// * [`ScrollNotificationObserver::maybe_of`], which is similar to this method,
    ///   but returns `None` if no [`ScrollNotificationObserver`] ancestor is found.
    pub fn of(app: &mut App, context: BuildContext) -> Handle<ScrollNotificationObserverState> {
        ScrollNotificationObserver::maybe_of(app, context).expect(
            "ScrollNotificationObserver.of() was called with a context that does not contain a \
             ScrollNotificationObserver widget.\n\
             No ScrollNotificationObserver widget ancestor could be found starting from the \
             context that was passed to ScrollNotificationObserver.of(). This can happen \
             because you are using a widget that looks for a ScrollNotificationObserver \
             ancestor, but no such ancestor exists.",
        )
    }
}

impl StatefulWidget for ScrollNotificationObserver {
    type State = ScrollNotificationObserverState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ScrollNotificationObserverState {
        ScrollNotificationObserverState {
            state: StateData::new(),
            listeners: Some(Vec::new()),
        }
    }
}

/// The listener list state for a [`ScrollNotificationObserver`] returned by
/// [`ScrollNotificationObserver::of`].
///
/// [`ScrollNotificationObserver`] is similar to
/// [`NotificationListener`]. It supports a listener list instead of
/// just a single listener and its listeners run unconditionally, they
/// do not require a gating boolean return value.
pub struct ScrollNotificationObserverState {
    state: StateData<ScrollNotificationObserver>,
    listeners: Option<Vec<ScrollNotificationCallback>>,
}

impl ScrollNotificationObserverState {
    fn debug_assert_not_disposed(self: Handle<Self>, app: &App) -> bool {
        debug_assert!(
            app.get(self).listeners.is_some(),
            "A ScrollNotificationObserverState was used after being disposed. Once you have \
             called dispose() on a ScrollNotificationObserverState, it can no longer be used."
        );
        true
    }

    /// Add a [`ScrollNotificationCallback`] that will be called each time
    /// a descendant scrolls.
    pub fn add_listener(self: Handle<Self>, app: &mut App, listener: ScrollNotificationCallback) {
        debug_assert!(self.debug_assert_not_disposed(app));
        app.get_mut(self)
            .listeners
            .as_mut()
            .expect("a live observer keeps its listeners")
            .push(listener);
    }

    /// Remove the specified [`ScrollNotificationCallback`].
    pub fn remove_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &ScrollNotificationCallback,
    ) {
        debug_assert!(self.debug_assert_not_disposed(app));
        let listeners = app
            .get_mut(self)
            .listeners
            .as_mut()
            .expect("a live observer keeps its listeners");
        if let Some(index) = listeners
            .iter()
            .position(|entry| Rc::ptr_eq(entry, listener))
        {
            listeners.remove(index);
        }
    }

    fn notify_listeners(self: Handle<Self>, app: &mut App, notification: &dyn ScrollNotification) {
        debug_assert!(self.debug_assert_not_disposed(app));
        let local_listeners = app
            .get(self)
            .listeners
            .clone()
            .expect("a live observer keeps its listeners");
        if local_listeners.is_empty() {
            return;
        }

        for entry in local_listeners {
            // Dart's `entry.list != null`: a listener removed by an earlier one is skipped.
            let still_registered = app.get(self).listeners.as_ref().is_some_and(|listeners| {
                listeners
                    .iter()
                    .any(|listener| Rc::ptr_eq(listener, &entry))
            });
            if still_registered {
                entry(app, notification);
            }
        }
    }
}

impl State for ScrollNotificationObserverState {
    type Widget = ScrollNotificationObserver;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let child = self.widget(app).child.clone();
        NotificationListener::<ScrollMetricsNotification>::new(
            NotificationListener::<dyn ScrollNotification>::new(
                ScrollNotificationObserverScope {
                    scroll_notification_observer_state: self,
                    child,
                }
                .into_widget(),
            )
            .on_notification(move |app, notification| {
                self.notify_listeners(app, notification);
                false
            })
            .into_widget(),
        )
        .on_notification(move |app, notification: &ScrollMetricsNotification| {
            // A `ScrollMetricsNotification` allows listeners to be notified for an
            // initial state, as well as if the content dimensions change without
            // scrolling.
            let update = notification.as_scroll_update();
            self.notify_listeners(app, &update);
            false
        })
        .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.debug_assert_not_disposed(app));
        app.get_mut(self).listeners = None;
    }
}

impl Debug for ScrollNotificationObserverState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScrollNotificationObserverState")
            .field(
                "listeners",
                &self.listeners.as_ref().map(|listeners| listeners.len()),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;

    use inset_painting::AxisDirection;

    use super::*;
    use crate::framework::{AnyElement, Element, Notification};
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;
    use crate::widgets::scroll_metrics::{FixedScrollMetrics, ScrollMetrics};
    use crate::widgets::scroll_notification::ScrollUpdateNotification;

    fn descendant(harness: &Harness, app: &App, depth: usize) -> AnyElement {
        let mut element = harness.root.as_element();
        for _ in 0..=depth {
            element = element.children(app)[0];
        }
        element
    }

    fn metrics() -> Rc<dyn ScrollMetrics> {
        Rc::new(FixedScrollMetrics::new(
            Some(0.0),
            Some(400.0),
            Some(10.0),
            Some(100.0),
            AxisDirection::Down,
            1.0,
        ))
    }

    /// Mounts the observer and returns the context of the widget under its scope.
    fn mount(app: &mut App) -> (Harness, BuildContext) {
        let harness = Harness::mount(
            app,
            ScrollNotificationObserver::new(SizedBox::shrink()).into_widget(),
        );
        harness.pump(app);
        // Stateful element, both notification listeners, the scope, then the child.
        let context = descendant(&harness, app, 4);
        (harness, context)
    }

    #[test]
    fn every_listener_sees_a_descendants_scroll_notification() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (_harness, context) = mount(&mut app);
        let observer = ScrollNotificationObserver::of(&mut app, context);

        let seen = Rc::new(RefCell::new(Vec::new()));
        for label in ["first", "second"] {
            let seen = seen.clone();
            observer.add_listener(
                &mut app,
                Rc::new(move |_app, notification| {
                    seen.borrow_mut()
                        .push((label, notification.metrics().pixels()));
                }),
            );
        }

        ScrollUpdateNotification::new(metrics(), context).dispatch(&mut app, Some(context));

        assert_eq!(*seen.borrow(), [("first", 10.0), ("second", 10.0)]);
    }

    #[test]
    fn a_metrics_notification_arrives_as_a_scroll_update() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (_harness, context) = mount(&mut app);
        let observer = ScrollNotificationObserver::of(&mut app, context);

        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorded = seen.clone();
        observer.add_listener(
            &mut app,
            Rc::new(move |_app, notification| {
                recorded
                    .borrow_mut()
                    .push(notification.metrics().max_scroll_extent());
            }),
        );

        ScrollMetricsNotification::new(metrics(), context).dispatch(&mut app, Some(context));

        assert_eq!(*seen.borrow(), [400.0]);
    }

    #[test]
    fn a_removed_listener_stops_hearing_notifications() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (_harness, context) = mount(&mut app);
        let observer = ScrollNotificationObserver::of(&mut app, context);

        let seen = Rc::new(RefCell::new(0));
        let recorded = seen.clone();
        let listener: ScrollNotificationCallback = Rc::new(move |_app, _notification| {
            *recorded.borrow_mut() += 1;
        });
        observer.add_listener(&mut app, listener.clone());

        ScrollUpdateNotification::new(metrics(), context).dispatch(&mut app, Some(context));
        assert_eq!(*seen.borrow(), 1);

        observer.remove_listener(&mut app, &listener);
        ScrollUpdateNotification::new(metrics(), context).dispatch(&mut app, Some(context));
        assert_eq!(*seen.borrow(), 1);
    }
}

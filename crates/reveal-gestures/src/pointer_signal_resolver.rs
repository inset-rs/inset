//! Flutter counterpart: `gestures/pointer_signal_resolver.dart`.

use std::rc::Rc;

use reveal_foundation::{App, Handle};

use crate::events::PointerEvent;

/// The callback to register with a [`PointerSignalResolver`] to express
/// interest in a pointer signal event.
pub type PointerSignalResolvedCallback = Rc<dyn Fn(&mut App, &PointerEvent)>;

/// Mediates disputes over which listener should handle pointer signal events
/// when multiple listeners wish to handle those events.
///
/// Pointer signals (such as [`crate::PointerScrollEvent`]) are immediate, so unlike
/// events that participate in the gesture arena, pointer signals always
/// resolve at the end of event dispatch. Yet if objects interested in handling
/// these signal events were to handle them directly, it would cause issues
/// such as multiple `Scrollable` widgets in the widget hierarchy responding
/// to the same mouse wheel event. Using this class, these events will only
/// be dispatched to the first registered handler, which will in turn
/// correspond to the widget that's deepest in the widget hierarchy.
///
/// To use this class, objects should register their event handler like so:
///
/// ```text
/// fn handle_signal_event(app: &mut App, event: &PointerEvent) {
///     let resolver = GestureBinding::instance(app).pointer_signal_resolver(app);
///     resolver.register(app, event, Rc::new(|app, event| {
///         // handle the event...
///     }));
/// }
/// ```
#[derive(Default)]
pub struct PointerSignalResolver {
    first_registered_callback: Option<PointerSignalResolvedCallback>,
    current_event: Option<PointerEvent>,
}

impl PointerSignalResolver {
    /// Creates a resolver with no event in flight.
    pub fn new(app: &mut App) -> Handle<PointerSignalResolver> {
        app.create(PointerSignalResolver::default())
    }

    /// Registers interest in handling `event`.
    ///
    /// This method may be called multiple times (typically from different parts
    /// of the widget hierarchy) for the same `event`, with different `callback`s,
    /// as the event is being dispatched across the tree. Once the dispatching is
    /// complete, the [`crate::GestureBinding`] calls [`resolve`](Self::resolve), and the
    /// first registered callback is called.
    ///
    /// The `callback` is invoked with one argument, the `event`.
    ///
    /// Once the [`register`](Self::register) method has been called with a particular
    /// `event`, it must not be called for other `event`s until after
    /// [`resolve`](Self::resolve) has been called. Only one event disambiguation can be in
    /// flight at a time. In normal use this is achieved by only registering callbacks for an
    /// event as it is actively being dispatched (for example, in `Listener.onPointerSignal`).
    pub fn register(
        self: Handle<Self>,
        app: &mut App,
        event: &PointerEvent,
        callback: PointerSignalResolvedCallback,
    ) {
        let resolver = app.get_mut(self);
        if resolver.first_registered_callback.is_some() {
            return;
        }
        resolver.current_event = Some(event.clone());
        resolver.first_registered_callback = Some(callback);
    }

    /// Resolves the event, calling the first registered callback if there was
    /// one.
    ///
    /// This is called by the [`crate::GestureBinding`] after the framework has finished
    /// dispatching the pointer signal event.
    pub fn resolve(self: Handle<Self>, app: &mut App) {
        let resolver = app.get_mut(self);
        let Some(callback) = resolver.first_registered_callback.take() else {
            debug_assert!(resolver.current_event.is_none());
            // Nothing in the framework/app wants to handle the event. Allowing the platform to
            // trigger its default native actions waits with `PointerEvent.respond`.
            return;
        };
        let event = resolver
            .current_event
            .take()
            .expect("a registered callback carries its event");
        callback(app, &event);
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;

    use crate::events::PointerScrollEvent;

    use super::*;

    #[test]
    fn only_the_first_registered_callback_runs() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let resolver = PointerSignalResolver::new(&mut app);
        let event = PointerEvent::Scroll(PointerScrollEvent::default());
        let called = Rc::new(RefCell::new(Vec::<&'static str>::new()));

        let first = called.clone();
        resolver.register(
            &mut app,
            &event,
            Rc::new(move |_app, _event| first.borrow_mut().push("first")),
        );
        let second = called.clone();
        resolver.register(
            &mut app,
            &event,
            Rc::new(move |_app, _event| second.borrow_mut().push("second")),
        );

        resolver.resolve(&mut app);
        assert_eq!(*called.borrow(), vec!["first"]);

        // The next event starts over.
        let third = called.clone();
        resolver.register(
            &mut app,
            &event,
            Rc::new(move |_app, _event| third.borrow_mut().push("third")),
        );
        resolver.resolve(&mut app);
        assert_eq!(*called.borrow(), vec!["first", "third"]);
    }

    #[test]
    fn resolving_without_a_registration_does_nothing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let resolver = PointerSignalResolver::new(&mut app);
        resolver.resolve(&mut app);
    }
}

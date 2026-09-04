//! Flutter counterpart: `gestures/pointer_router.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::IndexMap;
use reveal_embedder::Matrix4;
use reveal_foundation::{App, Handle, HandleId};

use crate::events::PointerEvent;

type RouteCallback = dyn Fn(&mut App, PointerEvent);

/// A callback that receives a [`PointerEvent`].
///
/// Flutter's counterpart is `typedef PointerRoute = void Function(PointerEvent event)`.
/// Receives [`App`] because a Rust route cannot own what it mutates. Identity
/// matches [`reveal_foundation::Listener`]: clones of [`PointerRoute::new`](PointerRoute::new), or a
/// rebuilt [`handle_method`](PointerRoute::handle_method) tear-off.
#[derive(Clone)]
pub struct PointerRoute {
    callback: Rc<RouteCallback>,
    identity: RouteIdentity,
}

#[derive(Clone, Copy)]
enum RouteIdentity {
    Closure,
    HandleMethod(HandleId, TypeId),
}

impl PointerRoute {
    /// Wraps a closure as a pointer route.
    pub fn new(callback: impl Fn(&mut App, PointerEvent) + 'static) -> PointerRoute {
        PointerRoute {
            callback: Rc::new(callback),
            identity: RouteIdentity::Closure,
        }
    }

    /// A handle's associated function, comparable like a Dart method tear-off.
    ///
    /// Pass the function by name. Rebuild at the removal site with the same
    /// handle and function.
    pub fn handle_method<T: 'static, F>(this: Handle<T>, f: F) -> PointerRoute
    where
        F: Fn(Handle<T>, &mut App, PointerEvent) + 'static,
    {
        const {
            assert!(
                std::mem::size_of::<F>() == 0,
                "pass the function by name: a fn pointer or capturing closure \
                 has no canonical identity to match on removal"
            )
        };
        PointerRoute {
            callback: Rc::new(move |app: &mut App, event| f(this, app, event)),
            identity: RouteIdentity::HandleMethod(this.id(), TypeId::of::<F>()),
        }
    }

    fn call(&self, app: &mut App, event: PointerEvent) {
        (self.callback)(app, event);
    }
}

impl PartialEq for PointerRoute {
    fn eq(&self, other: &PointerRoute) -> bool {
        match (self.identity, other.identity) {
            (RouteIdentity::Closure, RouteIdentity::Closure) => {
                Rc::ptr_eq(&self.callback, &other.callback)
            }
            (
                RouteIdentity::HandleMethod(handle, function),
                RouteIdentity::HandleMethod(other_handle, other_function),
            ) => handle == other_handle && function == other_function,
            _ => false,
        }
    }
}

impl Eq for PointerRoute {}

impl Hash for PointerRoute {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.identity {
            RouteIdentity::Closure => (Rc::as_ptr(&self.callback) as *const ()).hash(state),
            RouteIdentity::HandleMethod(handle, function) => {
                handle.hash(state);
                function.hash(state);
            }
        }
    }
}

impl Debug for PointerRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PointerRoute")
    }
}

type RouteTable = IndexMap<PointerRoute, Option<Matrix4>>;

/// A routing table for [`PointerEvent`] events.
pub struct PointerRouter {
    route_map: IndexMap<i64, RouteTable>,
    global_routes: RouteTable,
}

impl PointerRouter {
    /// Creates an empty router.
    pub fn new(app: &mut App) -> Handle<PointerRouter> {
        app.create(PointerRouter {
            route_map: IndexMap::new(),
            global_routes: IndexMap::new(),
        })
    }

    /// Adds a route to the routing table.
    ///
    /// Whenever this object routes a [`PointerEvent`] corresponding to
    /// pointer, call route.
    ///
    /// Routes added reentrantly within [`route`](Self::route) will take effect when
    /// routing the next event.
    pub fn add_route(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        route: PointerRoute,
        transform: Option<Matrix4>,
    ) {
        let routes = app.get_mut(self).route_map.entry(pointer).or_default();
        debug_assert!(!routes.contains_key(&route));
        routes.insert(route, transform);
    }

    /// Removes a route from the routing table.
    ///
    /// No longer call route when routing a [`PointerEvent`] corresponding to
    /// pointer. Requires that this route was previously added to the router.
    ///
    /// Routes removed reentrantly within [`route`](Self::route) will take effect
    /// immediately.
    pub fn remove_route(self: Handle<Self>, app: &mut App, pointer: i64, route: &PointerRoute) {
        let data = app.get_mut(self);
        debug_assert!(data.route_map.contains_key(&pointer));
        let routes = data.route_map.get_mut(&pointer).unwrap();
        debug_assert!(routes.contains_key(route));
        routes.shift_remove(route);
        if routes.is_empty() {
            data.route_map.shift_remove(&pointer);
        }
    }

    /// Adds a route to the global entry in the routing table.
    ///
    /// Whenever this object routes a [`PointerEvent`], call route.
    ///
    /// Routes added reentrantly within [`route`](Self::route) will take effect when
    /// routing the next event.
    pub fn add_global_route(
        self: Handle<Self>,
        app: &mut App,
        route: PointerRoute,
        transform: Option<Matrix4>,
    ) {
        let data = app.get_mut(self);
        debug_assert!(!data.global_routes.contains_key(&route));
        data.global_routes.insert(route, transform);
    }

    /// Removes a route from the global entry in the routing table.
    ///
    /// No longer call route when routing a [`PointerEvent`]. Requires that this route was previously added via [`add_global_route`](Self::add_global_route).
    ///
    /// Routes removed reentrantly within [`route`](Self::route) will take effect
    /// immediately.
    pub fn remove_global_route(self: Handle<Self>, app: &mut App, route: &PointerRoute) {
        let data = app.get_mut(self);
        debug_assert!(data.global_routes.contains_key(route));
        data.global_routes.shift_remove(route);
    }

    /// The number of global routes that have been registered.
    ///
    /// This is valid in debug builds only. In release builds, this will panic.
    pub fn debug_global_route_count(self: Handle<Self>, app: &App) -> usize {
        if cfg!(debug_assertions) {
            app.get(self).global_routes.len()
        } else {
            panic!("debugGlobalRouteCount is not supported in release builds");
        }
    }

    fn dispatch(
        app: &mut App,
        event: &PointerEvent,
        route: &PointerRoute,
        transform: Option<Matrix4>,
    ) {
        let event = event.transformed(transform);
        route.call(app, event);
    }

    /// Calls the routes registered for this pointer event.
    ///
    /// Routes are called in the order in which they were added to the
    /// PointerRouter object.
    pub fn route(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        let (pointer_routes, copied_global_routes) = {
            let data = app.get(self);
            (
                data.route_map.get(&event.pointer()).cloned(),
                data.global_routes.clone(),
            )
        };
        if let Some(routes) = pointer_routes {
            self.dispatch_event_to_routes(app, &event, routes, true);
        }
        self.dispatch_event_to_routes(app, &event, copied_global_routes, false);
    }

    fn dispatch_event_to_routes(
        self: Handle<Self>,
        app: &mut App,
        event: &PointerEvent,
        copied_routes: RouteTable,
        pointer_routes: bool,
    ) {
        for (route, transform) in copied_routes {
            let still_registered = {
                let data = app.get(self);
                if pointer_routes {
                    data.route_map
                        .get(&event.pointer())
                        .is_some_and(|routes| routes.contains_key(&route))
                } else {
                    data.global_routes.contains_key(&route)
                }
            };
            if still_registered {
                Self::dispatch(app, event, &route, transform);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    use reveal_embedder::Offset;
    use reveal_foundation::App;

    use crate::events::PointerDownEvent;

    fn down(pointer: i64) -> PointerEvent {
        PointerEvent::Down(PointerDownEvent {
            pointer,
            position: Offset::ZERO,
            ..PointerDownEvent::default()
        })
    }

    #[test]
    fn should_route_pointers() {
        let mut app = App::new();
        let ran = Rc::new(Cell::new(false));
        let ran_flag = Rc::clone(&ran);
        let callback = PointerRoute::new(move |_app, _event| {
            ran_flag.set(true);
        });

        let router = PointerRouter::new(&mut app);
        router.add_route(&mut app, 3, callback.clone(), None);
        router.route(&mut app, down(2));
        assert!(!ran.get());
        router.route(&mut app, down(3));
        assert!(ran.get());
        ran.set(false);
        router.remove_route(&mut app, 3, &callback);
        router.route(&mut app, down(3));
        assert!(!ran.get());
    }

    #[test]
    fn supports_reentrant_cancellation() {
        let mut app = App::new();
        let ran = Rc::new(Cell::new(false));
        let ran_flag = Rc::clone(&ran);
        let callback = PointerRoute::new(move |_app, _event| {
            ran_flag.set(true);
        });

        let router = PointerRouter::new(&mut app);
        let to_remove = callback.clone();
        router.add_route(
            &mut app,
            2,
            PointerRoute::new(move |app, _event| {
                router.remove_route(app, 2, &to_remove);
            }),
            None,
        );
        router.add_route(&mut app, 2, callback, None);
        router.route(&mut app, down(2));
        assert!(!ran.get());
    }

    #[test]
    fn supports_global_callbacks() {
        let mut app = App::new();
        let first_ran = Rc::new(Cell::new(false));
        let second_ran = Rc::new(Cell::new(false));
        let first_flag = Rc::clone(&first_ran);
        let second_flag = Rc::clone(&second_ran);

        let router = PointerRouter::new(&mut app);
        router.add_global_route(
            &mut app,
            PointerRoute::new(move |app, _event| {
                first_flag.set(true);
                let second_flag = Rc::clone(&second_flag);
                router.add_global_route(
                    app,
                    PointerRoute::new(move |_app, _event| {
                        second_flag.set(true);
                    }),
                    None,
                );
            }),
            None,
        );

        router.route(&mut app, down(2));
        assert!(first_ran.get());
        assert!(!second_ran.get());
    }

    #[test]
    fn supports_reentrant_global_cancellation() {
        let mut app = App::new();
        let ran = Rc::new(Cell::new(false));
        let ran_flag = Rc::clone(&ran);
        let callback = PointerRoute::new(move |_app, _event| {
            ran_flag.set(true);
        });

        let router = PointerRouter::new(&mut app);
        let to_remove = callback.clone();
        router.add_global_route(
            &mut app,
            PointerRoute::new(move |app, _event| {
                router.remove_global_route(app, &to_remove);
            }),
            None,
        );
        router.add_global_route(&mut app, callback, None);
        router.route(&mut app, down(2));
        assert!(!ran.get());
    }

    #[test]
    fn handle_method_tear_off_matches_on_remove() {
        let mut app = App::new();
        struct Flag(Cell<bool>);
        fn mark(this: Handle<Flag>, app: &mut App, _event: PointerEvent) {
            app.get_mut(this).0.set(true);
        }
        let flag = app.create(Flag(Cell::new(false)));
        let router = PointerRouter::new(&mut app);
        router.add_route(&mut app, 1, PointerRoute::handle_method(flag, mark), None);
        router.remove_route(&mut app, 1, &PointerRoute::handle_method(flag, mark));
        router.route(&mut app, down(1));
        assert!(!app.get(flag).0.get());
    }
}

//! Flutter counterpart: `gestures/binding.dart`.

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use reveal_embedder::{Offset, PointerDataPacket, ViewId};
use reveal_foundation::{App, Handle, Listener};

use crate::arena::GestureArenaManager;
use crate::converter::PointerEventConverter;
use crate::debug::{debug_print_hit_test_results, debug_print_mouse_hover_events};
use crate::events::{PointerCancelEvent, PointerEvent};
use crate::hit_test::{HitTestEntry, HitTestResult, HitTestTarget, HitTestable};
use crate::pointer_router::PointerRouter;

/// What Flutter's `RendererBinding` mixin adds on top of `GestureBinding`: it overrides
/// `hitTestInView` to walk the render trees before this binding adds itself, and the start of
/// `dispatchEvent` (the body before `super.dispatchEvent`) to update the mouse tracker.
///
/// Registered with [`GestureBinding::set_overrides`]. A binding that lives in the arena
/// implements [`GestureBindingOverridesObject`] instead; its `Handle` is then this trait.
pub trait GestureBindingOverrides: HitTestable {
    /// Runs at the start of [`GestureBinding::dispatch_event`], before the event is routed.
    fn will_dispatch_event(
        &self,
        app: &mut App,
        event: &PointerEvent,
        hit_test_result: Option<&HitTestResult>,
    );
}

/// The object side of [`GestureBindingOverrides`]: the binding struct implements this, and its
/// `Handle` is then [`HitTestable`] and [`GestureBindingOverrides`].
///
/// Those two take `&self` so [`GestureBinding`] can hold them erased; a crate above cannot
/// implement them for `Handle<ItsType>`, so it implements this one on the type instead.
pub trait GestureBindingOverridesObject: Sized + 'static {
    /// See [`HitTestable::hit_test`].
    fn hit_test(self: Handle<Self>, app: &mut App, result: &mut HitTestResult, position: Offset);

    /// See [`HitTestable::hit_test_in_view`].
    fn hit_test_in_view(
        self: Handle<Self>,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
        view_id: ViewId,
    );

    /// See [`GestureBindingOverrides::will_dispatch_event`].
    fn will_dispatch_event(
        self: Handle<Self>,
        app: &mut App,
        event: &PointerEvent,
        hit_test_result: Option<&HitTestResult>,
    );
}

impl<T: GestureBindingOverridesObject> HitTestable for Handle<T> {
    fn hit_test(&self, app: &mut App, result: &mut HitTestResult, position: Offset) {
        T::hit_test(*self, app, result, position);
    }

    fn hit_test_in_view(
        &self,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
        view_id: ViewId,
    ) {
        T::hit_test_in_view(*self, app, result, position, view_id);
    }
}

impl<T: GestureBindingOverridesObject> GestureBindingOverrides for Handle<T> {
    fn will_dispatch_event(
        &self,
        app: &mut App,
        event: &PointerEvent,
        hit_test_result: Option<&HitTestResult>,
    ) {
        T::will_dispatch_event(*self, app, event, hit_test_result);
    }
}

/// A binding for the gesture subsystem.
///
/// Dart's `GestureBinding.instance` is [`GestureBinding::instance`], the App's
/// singleton.
#[derive(Default)]
pub struct GestureBinding {
    pending_pointer_events: VecDeque<PointerEvent>,
    /// Filled on first [`GestureBinding::instance`]. `Default` cannot mint Handles.
    pointer_router: Option<Handle<PointerRouter>>,
    gesture_arena: Option<Handle<GestureArenaManager>>,
    hit_tests: HashMap<i64, HitTestResult>,
    /// Flutter's `hitTestInView` override point: the renderer binding registers itself here.
    overrides: Option<Rc<dyn GestureBindingOverrides>>,
}

impl GestureBinding {
    /// The current [`GestureBinding`], if one has been created.
    ///
    /// Dart's `GestureBinding.instance`.
    pub fn instance(app: &mut App) -> Handle<GestureBinding> {
        let this: Handle<GestureBinding> = app.singleton();
        if app.get(this).pointer_router.is_none() {
            let router = PointerRouter::new(app);
            let arena = GestureArenaManager::new(app);
            let binding = app.get_mut(this);
            binding.pointer_router = Some(router);
            binding.gesture_arena = Some(arena);
        }
        this
    }

    /// A router that routes all pointer events received from the engine.
    pub fn pointer_router(self: Handle<Self>, app: &App) -> Handle<PointerRouter> {
        app.get(self)
            .pointer_router
            .expect("GestureBinding::instance must run first")
    }

    /// The gesture arenas used for disambiguating the meaning of sequences of
    /// pointer events.
    pub fn gesture_arena(self: Handle<Self>, app: &App) -> Handle<GestureArenaManager> {
        app.get(self)
            .gesture_arena
            .expect("GestureBinding::instance must run first")
    }

    /// Converts a host packet into framework events and flushes them.
    pub fn handle_pointer_data_packet(
        self: Handle<Self>,
        app: &mut App,
        packet: PointerDataPacket,
    ) {
        let platform = app.platform();
        let events: Vec<PointerEvent> = PointerEventConverter::expand(packet.data, |view_id| {
            platform
                .view(view_id)
                .map(|view| view.metrics().device_pixel_ratio)
        })
        .collect();
        app.get_mut(self).pending_pointer_events.extend(events);
        self.flush_pointer_event_queue(app);
    }

    /// Dispatch a [`PointerCancelEvent`] for the given pointer soon.
    ///
    /// The pointer event will be dispatched before the next pointer event and
    /// before the end of the microtask but not within this function call.
    pub fn cancel_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        if app.get(self).pending_pointer_events.is_empty() {
            app.schedule_microtask(Listener::handle_method(
                self,
                GestureBinding::flush_pointer_event_queue,
            ));
        }
        app.get_mut(self)
            .pending_pointer_events
            .push_front(PointerEvent::Cancel(PointerCancelEvent {
                pointer,
                ..PointerCancelEvent::default()
            }));
    }

    fn flush_pointer_event_queue(self: Handle<Self>, app: &mut App) {
        while let Some(event) = app.get_mut(self).pending_pointer_events.pop_front() {
            self.handle_pointer_event(app, event);
        }
    }

    /// Dispatch an event to the targets found by a hit test on its position.
    pub fn handle_pointer_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        self.handle_pointer_event_immediately(app, event);
    }

    fn handle_pointer_event_immediately(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        let pointer = event.pointer();
        let is_down_or_pan_zoom_start =
            matches!(event, PointerEvent::Down(_) | PointerEvent::PanZoomStart(_));
        let is_up_or_cancel_or_pan_zoom_end = matches!(
            event,
            PointerEvent::Up(_) | PointerEvent::Cancel(_) | PointerEvent::PanZoomEnd(_)
        );
        let is_new_hit_test = matches!(
            event,
            PointerEvent::Down(_)
                | PointerEvent::Hover(_)
                | PointerEvent::Scroll(_)
                | PointerEvent::ScrollInertiaCancel(_)
                | PointerEvent::Scale(_)
                | PointerEvent::PanZoomStart(_)
        );

        let (hit_test_result, cache_after) = if is_new_hit_test {
            debug_assert!(
                !app.get(self).hit_tests.contains_key(&pointer),
                "Pointer unexpectedly has a HitTestResult associated with it."
            );
            let mut hit_test_result = HitTestResult::new();
            self.hit_test_in_view(app, &mut hit_test_result, event.position(), event.view_id());
            (Some(hit_test_result), is_down_or_pan_zoom_start)
        } else if is_up_or_cancel_or_pan_zoom_end {
            (app.get_mut(self).hit_tests.remove(&pointer), false)
        } else if event.down() || matches!(event, PointerEvent::PanZoomUpdate(_)) {
            (app.get_mut(self).hit_tests.remove(&pointer), true)
        } else {
            (None, false)
        };

        if cfg!(debug_assertions)
            && debug_print_hit_test_results()
            && let Some(result) = &hit_test_result
        {
            eprintln!("{event:?}: {result:?}");
        }
        if cfg!(debug_assertions)
            && debug_print_mouse_hover_events()
            && matches!(event, PointerEvent::Hover(_))
        {
            eprintln!("{event:?}");
        }

        if hit_test_result.is_some()
            || matches!(event, PointerEvent::Added(_) | PointerEvent::Removed(_))
        {
            self.dispatch_event(app, event, hit_test_result.as_ref());
        }
        if cache_after && let Some(result) = hit_test_result {
            app.get_mut(self).hit_tests.insert(pointer, result);
        }
    }

    /// Registers the binding layered above this one, as Flutter's `RendererBinding` does by
    /// overriding [`hit_test_in_view`](Self::hit_test_in_view) and
    /// [`dispatch_event`](Self::dispatch_event).
    pub fn set_overrides(
        self: Handle<Self>,
        app: &mut App,
        overrides: Rc<dyn GestureBindingOverrides>,
    ) {
        app.get_mut(self).overrides = Some(overrides);
    }

    /// Determine which [`HitTestTarget`] objects are located at a given position in the
    /// specified view.
    pub fn hit_test_in_view(
        self: Handle<Self>,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
        view_id: ViewId,
    ) {
        let overrides = app.get(self).overrides.clone();
        if let Some(overrides) = overrides {
            overrides.hit_test_in_view(app, result, position, view_id);
        }
        result.add(HitTestEntry::new(self));
    }

    /// Dispatch an event to [`pointer_router`](Self::pointer_router) and the path of a hit test result.
    ///
    /// The `hit_test_result` argument may only be none for [`PointerEvent::Added`]
    /// or [`PointerEvent::Removed`].
    pub fn dispatch_event(
        self: Handle<Self>,
        app: &mut App,
        event: PointerEvent,
        hit_test_result: Option<&HitTestResult>,
    ) {
        let overrides = app.get(self).overrides.clone();
        if let Some(overrides) = overrides {
            overrides.will_dispatch_event(app, &event, hit_test_result);
        }
        let Some(hit_test_result) = hit_test_result else {
            debug_assert!(matches!(
                event,
                PointerEvent::Added(_) | PointerEvent::Removed(_)
            ));
            let router = self.pointer_router(app);
            router.route(app, event);
            return;
        };
        for entry in hit_test_result.path() {
            entry
                .target()
                .handle_event(app, &event.transformed(entry.transform()), entry);
        }
    }

    /// Reset states of [`GestureBinding`].
    ///
    /// This clears the hit test records.
    ///
    /// This is typically called between tests.
    pub fn reset_gesture_binding(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).hit_tests.clear();
    }
}

impl HitTestTarget for Handle<GestureBinding> {
    fn handle_event(&self, app: &mut App, event: &PointerEvent, _entry: &HitTestEntry) {
        let this = *self;
        let router = this.pointer_router(app);
        router.route(app, event.clone());
        let arena = this.gesture_arena(app);
        if matches!(event, PointerEvent::Down(_) | PointerEvent::PanZoomStart(_)) {
            arena.close(app, event.pointer());
        } else if matches!(event, PointerEvent::Up(_) | PointerEvent::PanZoomEnd(_)) {
            arena.sweep(app, event.pointer());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::events::{PointerDownEvent, PointerMoveEvent, PointerUpEvent};
    use crate::pointer_router::PointerRoute;

    #[test]
    fn down_routes_through_the_binding_and_closes_the_arena() {
        let mut app = App::new();
        let binding = GestureBinding::instance(&mut app);
        let ran = Rc::new(Cell::new(0));
        let ran_flag = Rc::clone(&ran);
        binding.pointer_router(&app).add_route(
            &mut app,
            1,
            PointerRoute::new(move |_app, _event| {
                ran_flag.set(ran_flag.get() + 1);
            }),
            None,
        );

        binding.handle_pointer_event(
            &mut app,
            PointerEvent::Down(PointerDownEvent {
                pointer: 1,
                ..PointerDownEvent::default()
            }),
        );
        assert_eq!(ran.get(), 1);

        binding.handle_pointer_event(
            &mut app,
            PointerEvent::Move(PointerMoveEvent {
                pointer: 1,
                down: true,
                ..PointerMoveEvent::default()
            }),
        );
        assert_eq!(ran.get(), 2);

        binding.handle_pointer_event(
            &mut app,
            PointerEvent::Up(PointerUpEvent {
                pointer: 1,
                ..PointerUpEvent::default()
            }),
        );
        assert_eq!(ran.get(), 3);
    }
}

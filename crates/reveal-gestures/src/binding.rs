//! Flutter counterpart: `gestures/binding.dart`.

use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Offset, PointerDataPacket, ViewId};
use reveal_foundation::{App, Handle, Listener};

use crate::arena::GestureArenaManager;
use crate::converter::PointerEventConverter;
use crate::debug::debug_print_hit_test_results;
use crate::events::PointerEvent;
use crate::hit_test::{HitTestEntry, HitTestResult, HitTestTarget, HitTestable};
use crate::pointer_router::PointerRouter;

/// Slot data for [`GestureBinding`].
#[derive(Default)]
pub(crate) struct GestureBindingData {
    pending_pointer_events: VecDeque<PointerEvent>,
    /// Filled on first [`GestureBinding::instance`]. `Default` cannot mint Handles.
    pointer_router: Option<PointerRouter>,
    gesture_arena: Option<GestureArenaManager>,
    hit_tests: HashMap<i64, HitTestResult>,
    /// Flutter's `hitTestInView` override point: the renderer binding registers itself here.
    hit_testable: Option<Rc<dyn HitTestable>>,
}

/// A binding for the gesture subsystem.
///
/// Dart's `GestureBinding.instance` is [`GestureBinding::instance`], the App's
/// singleton; members take `&mut App`.
#[derive(Clone, Copy)]
pub struct GestureBinding(Handle<GestureBindingData>);

impl GestureBinding {
    /// The current [`GestureBinding`], if one has been created.
    ///
    /// Dart's `GestureBinding.instance`.
    pub fn instance(app: &mut App) -> GestureBinding {
        let this = GestureBinding(app.singleton());
        if app.get(this.0).pointer_router.is_none() {
            let router = PointerRouter::new(app);
            let arena = GestureArenaManager::new(app);
            let data = app.get_mut(this.0);
            data.pointer_router = Some(router);
            data.gesture_arena = Some(arena);
        }
        this
    }

    /// A router that routes all pointer events received from the engine.
    pub fn pointer_router(self, app: &App) -> PointerRouter {
        app.get(self.0)
            .pointer_router
            .expect("GestureBinding::instance must run first")
    }

    /// The gesture arenas used for disambiguating the meaning of sequences of
    /// pointer events.
    pub fn gesture_arena(self, app: &App) -> GestureArenaManager {
        app.get(self.0)
            .gesture_arena
            .expect("GestureBinding::instance must run first")
    }

    /// Converts a host packet into framework events and flushes them.
    pub fn handle_pointer_data_packet(app: &mut App, packet: PointerDataPacket) {
        let this = GestureBinding::instance(app);
        let platform = app.platform();
        let events: Vec<PointerEvent> = PointerEventConverter::expand(packet.data, |view_id| {
            platform
                .view(view_id)
                .map(|view| view.metrics().device_pixel_ratio)
        })
        .collect();
        app.get_mut(this.0).pending_pointer_events.extend(events);
        GestureBinding::flush_pointer_event_queue(app);
    }

    /// Dispatch a [`PointerCancelEvent`] for the given pointer soon.
    ///
    /// The pointer event will be dispatched before the next pointer event and
    /// before the end of the microtask but not within this function call.
    pub fn cancel_pointer(app: &mut App, pointer: i64) {
        let this = GestureBinding::instance(app);
        let was_empty = app.get(this.0).pending_pointer_events.is_empty();
        if was_empty {
            app.schedule_microtask(Listener::new(GestureBinding::flush_pointer_event_queue));
        }
        app.get_mut(this.0)
            .pending_pointer_events
            .push_front(PointerEvent::Cancel(crate::events::PointerCancelEvent {
                pointer,
                ..crate::events::PointerCancelEvent::default()
            }));
    }

    fn flush_pointer_event_queue(app: &mut App) {
        let this = GestureBinding::instance(app);
        loop {
            let event = app.get_mut(this.0).pending_pointer_events.pop_front();
            let Some(event) = event else {
                break;
            };
            GestureBinding::handle_pointer_event(app, event);
        }
    }

    /// Dispatch an event to the targets found by a hit test on its position.
    pub fn handle_pointer_event(app: &mut App, event: PointerEvent) {
        GestureBinding::handle_pointer_event_immediately(app, event);
    }

    fn handle_pointer_event_immediately(app: &mut App, event: PointerEvent) {
        let this = GestureBinding::instance(app);
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
                !app.get(this.0).hit_tests.contains_key(&pointer),
                "Pointer unexpectedly has a HitTestResult associated with it."
            );
            let mut hit_test_result = HitTestResult::new();
            this.hit_test_in_view(app, &mut hit_test_result, event.position(), event.view_id());
            (Some(hit_test_result), is_down_or_pan_zoom_start)
        } else if is_up_or_cancel_or_pan_zoom_end {
            (app.get_mut(this.0).hit_tests.remove(&pointer), false)
        } else if event.down() || matches!(event, PointerEvent::PanZoomUpdate(_)) {
            (app.get_mut(this.0).hit_tests.remove(&pointer), true)
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
            && crate::debug::debug_print_mouse_hover_events()
            && matches!(event, PointerEvent::Hover(_))
        {
            eprintln!("{event:?}");
        }

        if hit_test_result.is_some()
            || matches!(event, PointerEvent::Added(_) | PointerEvent::Removed(_))
        {
            this.dispatch_event(app, event, hit_test_result.as_ref());
        }
        if cache_after && let Some(result) = hit_test_result {
            app.get_mut(this.0).hit_tests.insert(pointer, result);
        }
    }

    /// Determine which [`HitTestTarget`] objects are located at a given position in
    /// the specified view.
    ///
    /// Flutter's `RendererBinding` overrides this to walk the render tree, then
    /// calls super (this method) to add the binding itself. Until that binding
    /// exists, the path is only this binding.
    /// Registers the object that hit-tests the render trees, as Flutter's `RendererBinding`
    /// does by overriding [`hit_test_in_view`](Self::hit_test_in_view).
    pub fn set_hit_testable(app: &mut App, hit_testable: Rc<dyn HitTestable>) {
        let this = GestureBinding::instance(app);
        app.get_mut(this.0).hit_testable = Some(hit_testable);
    }

    /// Determine which [`HitTestTarget`] objects are located at a given position in the
    /// specified view.
    pub fn hit_test_in_view(
        self,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
        view_id: ViewId,
    ) {
        let hit_testable = app.get(self.0).hit_testable.clone();
        if let Some(hit_testable) = hit_testable {
            hit_testable.hit_test_in_view(app, result, position, view_id);
        }
        result.add(HitTestEntry::new(self));
    }

    /// Dispatch an event to [`pointer_router`](Self::pointer_router) and the path of a hit test result.
    ///
    /// The `hit_test_result` argument may only be none for [`PointerEvent::Added`]
    /// or [`PointerEvent::Removed`].
    pub fn dispatch_event(
        self,
        app: &mut App,
        event: PointerEvent,
        hit_test_result: Option<&HitTestResult>,
    ) {
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
    pub fn reset_gesture_binding(app: &mut App) {
        let this = GestureBinding::instance(app);
        app.get_mut(this.0).hit_tests.clear();
    }
}

impl HitTestTarget for GestureBinding {
    fn handle_event(&self, app: &mut App, event: &PointerEvent, _entry: &HitTestEntry) {
        let router = self.pointer_router(app);
        router.route(app, event.clone());
        let arena = self.gesture_arena(app);
        if matches!(event, PointerEvent::Down(_) | PointerEvent::PanZoomStart(_)) {
            arena.close(app, event.pointer());
        } else if matches!(event, PointerEvent::Up(_) | PointerEvent::PanZoomEnd(_)) {
            arena.sweep(app, event.pointer());
        }
    }
}

impl Debug for GestureBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GestureBinding")
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

        GestureBinding::handle_pointer_event(
            &mut app,
            PointerEvent::Down(PointerDownEvent {
                pointer: 1,
                ..PointerDownEvent::default()
            }),
        );
        assert_eq!(ran.get(), 1);

        GestureBinding::handle_pointer_event(
            &mut app,
            PointerEvent::Move(PointerMoveEvent {
                pointer: 1,
                down: true,
                ..PointerMoveEvent::default()
            }),
        );
        assert_eq!(ran.get(), 2);

        GestureBinding::handle_pointer_event(
            &mut app,
            PointerEvent::Up(PointerUpEvent {
                pointer: 1,
                ..PointerUpEvent::default()
            }),
        );
        assert_eq!(ran.get(), 3);
    }
}

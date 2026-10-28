//! The one [`EmbedderClient`] the framework provides: owns the [`AppCell`] and
//! translates host pushes into binding methods.
//!
//! Dart has no type for this — the engine owns the isolate. [`App`] stays in
//! foundation. This crate sits above scheduler and gestures so the isolate
//! can name both.
#![feature(arbitrary_self_types)]

use std::cell::RefMut;
use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{
    AppExitReply, AppLifecycleState, ClipboardEvent, DropData, EditCommand, EmbedderClient, Frame,
    Instant, KeyData, MenuId, PlatformRef, PointerDataPacket, RequestId, TextEditingValue,
    TextInputAction, TrayIconEvent, TrayIconId, ViewError, ViewFocusEvent, ViewId,
};
use inset_foundation::{App, AppCell};
use inset_gestures::GestureBinding;
use inset_painting::PaintingBinding;
use inset_rendering::RendererBinding;
use inset_scheduler::{FrameTarget, SchedulerBinding};
use inset_services::{KeyEventManager, ServicesBinding, TextInput};

/// Host-facing isolate: the [`AppCell`] plus the methods the embedder pushes.
///
/// Every push is one call into the app: the `App` is borrowed for the binding call and
/// released before the cell's checkpoint, where the microtasks and futures the event queued
/// run.
pub struct Shell {
    app: Rc<AppCell>,
    platform: PlatformRef,
    /// The platform time the app clock was last advanced to.
    clock: Duration,
    /// The platform's `now` at its `elapsed` zero, known once a frame has reported its `elapsed`; lets every push, not only a frame or wake, move the app clock to the platform's.
    origin: Option<Instant>,
}

impl Shell {
    /// Builds the heap from `platform`, then runs application setup.
    ///
    /// Setup (later widgets `run_app`) requests the first frame when it
    /// installs work that needs one.
    pub fn new(platform: PlatformRef, setup: impl FnOnce(&mut App)) -> Shell {
        let shell = Shell {
            app: AppCell::with_platform(platform.clone()),
            platform,
            clock: Duration::ZERO,
            origin: None,
        };
        shell.handle_event(|app| {
            // Flutter's engine collects the platform's fonts before the framework runs.
            PaintingBinding::instance(app).install_platform_fonts(app);
            setup(app);
        });
        shell
    }

    /// The `App`, borrowed until the guard drops. A host that needs the whole thing, a borrow
    /// and then the checkpoint, takes [`cell`](Self::cell).
    pub fn app(&self) -> RefMut<'_, App> {
        self.app.borrow_mut()
    }

    pub fn cell(&self) -> &Rc<AppCell> {
        &self.app
    }

    /// One platform event: `f` on the borrowed `App`, then the checkpoint.
    fn handle_event<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        let result = f(&mut self.app.borrow_mut());
        self.app.checkpoint();
        result
    }

    /// An input or platform push: the app clock catches up with the platform's first, as Dart's
    /// timers measure real time from the moment they are created, then `f` runs.
    fn push<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> R {
        if let Some(origin) = self.origin {
            let elapsed = self.platform.now().saturating_duration_since(origin);
            self.advance_clock(elapsed);
        }
        self.handle_event(f)
    }

    /// Moves the app clock up to the platform's `elapsed`, firing the timers that came due.
    fn advance_clock(&mut self, elapsed: Duration) {
        if elapsed <= self.clock {
            return;
        }
        self.app.elapse(elapsed - self.clock);
        self.clock = elapsed;
    }

    /// dart:ui's `onMetricsChanged`, which the framework sets to
    /// `RendererBinding.handleMetricsChanged`. `_addView`, `_removeView` and `_updateDisplays`
    /// invoke it as `_updateWindowMetrics` does: a view coming or going, and a change to the
    /// screens, reach the app only this way.
    fn metrics_changed(&mut self) {
        self.push(|app| RendererBinding::instance(app).handle_metrics_changed(app));
    }
}

impl EmbedderClient for Shell {
    fn frame(&mut self, views: &[ViewId], frame: Frame) {
        self.origin = Some(self.platform.now() - frame.elapsed);
        self.advance_clock(frame.elapsed);
        // The engine runs `_beginFrame` and `_drawFrame` as two native tasks: two calls.
        self.handle_event(|app| SchedulerBinding::on_begin_frame(app, views, Some(frame.elapsed)));
        self.handle_event(|app| SchedulerBinding::on_draw_frame(app, views));
    }

    /// The framework hears which view came, then "metrics changed", as `_addView` invokes it.
    fn view_added(&mut self, id: ViewId) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_view_added.clone();
            if let Some(callback) = callback {
                callback(app, id);
            }
        });
        self.metrics_changed();
    }

    fn view_metrics_changed(&mut self, _id: ViewId) {
        self.metrics_changed();
    }

    /// The framework hears which view went, then "metrics changed", as `_removeView` invokes
    /// it.
    fn view_removed(&mut self, id: ViewId) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_view_removed.clone();
            if let Some(callback) = callback {
                callback(app, id);
            }
        });
        self.metrics_changed();
    }

    /// "Metrics changed", as `_updateDisplays` invokes it.
    fn screens_changed(&mut self) {
        self.metrics_changed();
    }

    fn view_focus_changed(&mut self, event: ViewFocusEvent) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_view_focus_change.clone();
            if let Some(callback) = callback {
                callback(app, event);
            }
        });
    }

    fn drop_data(&mut self, data: DropData) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_drop.clone();
            if let Some(callback) = callback {
                callback(app, data);
            }
        });
    }

    fn clipboard_event(&mut self, view: ViewId, event: ClipboardEvent) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_clipboard_event.clone();
            if let Some(callback) = callback {
                callback(app, view, event);
            }
        });
    }

    fn edit_command(&mut self, view: ViewId, command: EditCommand) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_edit_command.clone();
            if let Some(callback) = callback {
                callback(app, view, command);
            }
        });
    }

    fn menu_selected(&mut self, id: MenuId) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_menu_selected.clone();
            if let Some(callback) = callback {
                callback(app, id);
            }
        });
    }

    fn dock_menu_selected(&mut self, id: MenuId) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_dock_menu_selected.clone();
            if let Some(callback) = callback {
                callback(app, id);
            }
        });
    }

    fn tray_icon_event(&mut self, icon: TrayIconId, event: TrayIconEvent) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_tray_icon_event.clone();
            if let Some(callback) = callback {
                callback(app, icon, event);
            }
        });
    }

    fn pointer_data_packet(&mut self, packet: PointerDataPacket) {
        self.push(|app| GestureBinding::instance(app).handle_pointer_data_packet(app, packet));
    }

    fn key_data(&mut self, data: KeyData) -> bool {
        self.push(|app| KeyEventManager::instance(app).handle_key_data(app, data))
    }

    fn pop_route(&mut self) -> bool {
        self.push(|app| {
            let callback = app.platform_callbacks().on_pop_route.clone();
            callback.is_some_and(|callback| callback(app))
        })
    }

    fn view_close_requested(&mut self, view: ViewId) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_view_close_requested.clone();
            if let Some(callback) = callback {
                callback(app, view);
            }
        });
    }

    fn window_changed(&mut self, view: ViewId) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_window_changed.clone();
            if let Some(callback) = callback {
                callback(app, view);
            }
        });
    }

    fn view_request_failed(&mut self, request: RequestId, error: ViewError) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_view_request_failed.clone();
            if let Some(callback) = callback {
                callback(app, request, error);
            }
        });
    }

    /// The binding's answer goes to `reply` at the checkpoint of the call in which the
    /// observers decide: this push's own when they answer at once.
    fn request_app_exit(&mut self, reply: AppExitReply) {
        self.push(|app| {
            let answer = ServicesBinding::instance(app).handle_request_app_exit(app);
            app.spawn(async move |_cx| reply.answer(answer.await));
        });
    }

    /// On `resumed` or `inactive`, every view is marked to present and asked for a frame before
    /// the bindings hear the state: a view that did not change while the app was away would
    /// otherwise present nothing.
    fn app_lifecycle_state_changed(&mut self, state: AppLifecycleState) {
        self.push(|app| {
            if matches!(
                state,
                AppLifecycleState::Resumed | AppLifecycleState::Inactive
            ) {
                RendererBinding::instance(app).handle_mark_all_views_need_render(app);
                SchedulerBinding::schedule_forced_frame(app, FrameTarget::EveryView);
            }
            ServicesBinding::instance(app).handle_lifecycle_message(app, state);
        });
    }

    fn platform_brightness_changed(&mut self) {
        self.push(|app| {
            let callback = app
                .platform_callbacks()
                .on_platform_brightness_changed
                .clone();
            if let Some(callback) = callback {
                callback.call(app);
            }
        });
    }

    fn locales_changed(&mut self) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_locale_changed.clone();
            if let Some(callback) = callback {
                callback.call(app);
            }
        });
    }

    fn system_fonts_changed(&mut self) {
        self.push(|app| {
            PaintingBinding::instance(app).handle_system_fonts_did_change(app);
        });
    }

    fn clipboard_changed(&mut self) {
        self.push(|app| ServicesBinding::instance(app).handle_clipboard_changed(app));
    }

    fn wake(&mut self, elapsed: Duration) {
        self.advance_clock(elapsed);
        // A wake with no time behind it is a post from a native callback.
        self.app.wake();
    }

    fn text_input_editing_value(&mut self, _view: ViewId, value: TextEditingValue) {
        self.push(|app| TextInput::instance(app).update_editing_value(app, value));
    }

    fn text_input_action(&mut self, _view: ViewId, action: TextInputAction) {
        self.push(|app| TextInput::instance(app).perform_action(app, action));
    }

    fn text_input_closed(&mut self, _view: ViewId) {
        self.push(|app| TextInput::instance(app).connection_closed(app));
    }

    fn system_context_menu_hidden(&mut self) {
        self.push(inset_services::SystemContextMenuController::dispatch_system_hide);
    }

    fn custom_context_menu_action(&mut self, callback_id: &str) {
        let callback_id = callback_id.to_string();
        self.push(move |app| {
            inset_services::SystemContextMenuController::dispatch_custom_action(app, &callback_id);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use inset_embedder::{
        AppLifecycleState, EmbedderClient, Frame, KeyData, KeyEventType, PointerChange,
        PointerData, PointerDataPacket, PointerDeviceKind, ViewId,
    };
    use inset_foundation::App;
    use inset_gestures::{GestureBinding, PointerRoute};
    use inset_scheduler::{FrameTarget, SchedulerBinding};
    use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey, PhysicalKeyboardKey};
    use inset_test::{IMPLICIT_VIEW_ID, TestPlatform};

    use super::Shell;

    /// A platform with the one view a pointer packet, a focus event or a frame is addressed to.
    fn platform_with_a_view() -> TestPlatform {
        TestPlatform::new().with_views(&[IMPLICIT_VIEW_ID])
    }

    #[test]
    fn drop_data_reaches_the_platform_callback() {
        use std::cell::RefCell;
        use std::path::PathBuf;

        use inset_embedder::{DropChange, DropData};

        let seen = Rc::new(RefCell::new(None));
        let seen_slot = Rc::clone(&seen);
        let platform = Rc::new(platform_with_a_view());
        let mut shell = Shell::new(platform, move |app: &mut App| {
            app.platform_callbacks_mut().on_drop = Some(Rc::new(move |_app, data| {
                *seen_slot.borrow_mut() = Some(data);
            }));
        });
        shell.drop_data(DropData {
            view_id: IMPLICIT_VIEW_ID,
            change: DropChange::Dropped,
            physical_position: None,
            paths: vec![PathBuf::from("/a.wasm")],
        });
        let seen = seen.borrow();
        let seen = seen.as_ref().expect("the callback ran");
        assert_eq!(seen.change, DropChange::Dropped);
        assert_eq!(seen.paths, [PathBuf::from("/a.wasm")]);
    }

    #[test]
    fn a_clipboard_event_reaches_the_platform_callback_with_its_view() {
        use inset_embedder::{ClipboardEvent, CopyRequest};

        let seen = Rc::new(Cell::new(None));
        let seen_slot = Rc::clone(&seen);
        let mut shell = Shell::new(Rc::new(platform_with_a_view()), move |app: &mut App| {
            app.platform_callbacks_mut().on_clipboard_event =
                Some(Rc::new(move |_app, view, event| {
                    seen_slot.set(Some((view, matches!(event, ClipboardEvent::Cut(_)))));
                }));
        });
        let request = CopyRequest::new(|_items, _options| Box::pin(std::future::ready(Ok(()))));
        shell.clipboard_event(IMPLICIT_VIEW_ID, ClipboardEvent::Cut(request));
        assert_eq!(seen.get(), Some((IMPLICIT_VIEW_ID, true)));
    }

    #[test]
    fn menu_choices_edit_commands_and_tray_icon_events_reach_their_callbacks() {
        use std::cell::RefCell;

        use inset_embedder::{EditCommand, Offset, TrayIconEvent};

        let seen = Rc::new(RefCell::new(Vec::new()));
        let slot = Rc::clone(&seen);
        let mut shell = Shell::new(Rc::new(platform_with_a_view()), move |app: &mut App| {
            let callbacks = app.platform_callbacks_mut();
            let menu = Rc::clone(&slot);
            callbacks.on_menu_selected = Some(Rc::new(move |_app, id| {
                menu.borrow_mut().push(format!("menu {id}"));
            }));
            let dock = Rc::clone(&slot);
            callbacks.on_dock_menu_selected = Some(Rc::new(move |_app, id| {
                dock.borrow_mut().push(format!("dock {id}"));
            }));
            let edit = Rc::clone(&slot);
            callbacks.on_edit_command = Some(Rc::new(move |_app, view, command| {
                edit.borrow_mut().push(format!("{view:?} {command:?}"));
            }));
            let tray = Rc::clone(&slot);
            callbacks.on_tray_icon_event = Some(Rc::new(move |_app, icon, event| {
                tray.borrow_mut().push(format!("tray {icon} {event:?}"));
            }));
        });
        shell.menu_selected(3);
        shell.dock_menu_selected(4);
        shell.edit_command(IMPLICIT_VIEW_ID, EditCommand::Undo);
        shell.tray_icon_event(
            7,
            TrayIconEvent::Activated {
                position: Offset::new(1.0, 2.0),
            },
        );
        assert_eq!(
            *seen.borrow(),
            [
                "menu 3".to_owned(),
                "dock 4".to_owned(),
                format!("{IMPLICIT_VIEW_ID:?} Undo"),
                format!(
                    "tray 7 {:?}",
                    TrayIconEvent::Activated {
                        position: Offset::new(1.0, 2.0)
                    }
                ),
            ]
        );
    }

    #[test]
    fn a_change_of_the_clipboard_reaches_the_services_bindings_listenable() {
        use inset_foundation::{Listenable, Listener};
        use inset_services::ServicesBinding;

        let heard = Rc::new(Cell::new(0));
        let counter = Rc::clone(&heard);
        let mut shell = Shell::new(Rc::new(TestPlatform::new()), move |app: &mut App| {
            let clipboard = ServicesBinding::instance(app).clipboard(app);
            clipboard.add_listener(
                app,
                Listener::new(move |_app| counter.set(counter.get() + 1)),
            );
        });
        shell.clipboard_changed();
        assert_eq!(heard.get(), 1);
    }

    /// A reply to `shell.request_app_exit`, and the answer it was given so far.
    fn recorded_reply() -> (
        inset_embedder::AppExitReply,
        Rc<Cell<Option<inset_embedder::AppExitResponse>>>,
    ) {
        let answer = Rc::new(Cell::new(None));
        let given = Rc::clone(&answer);
        let reply = inset_embedder::AppExitReply::new(move |response| given.set(Some(response)));
        (reply, answer)
    }

    #[test]
    fn a_view_added_or_removed_reaches_the_framework_with_its_id() {
        let heard = Rc::new(std::cell::RefCell::new(Vec::new()));
        let (added, removed) = (Rc::clone(&heard), Rc::clone(&heard));
        let mut shell = Shell::new(Rc::new(TestPlatform::new()), move |app: &mut App| {
            let callbacks = app.platform_callbacks_mut();
            callbacks.on_view_added = Some(Rc::new(move |_app, view| {
                added.borrow_mut().push(("added", view))
            }));
            callbacks.on_view_removed = Some(Rc::new(move |_app, view| {
                removed.borrow_mut().push(("removed", view))
            }));
        });
        shell.view_added(ViewId(4));
        shell.view_removed(ViewId(4));
        assert_eq!(
            *heard.borrow(),
            [("added", ViewId(4)), ("removed", ViewId(4))]
        );
    }

    #[test]
    fn a_view_close_request_reaches_the_framework() {
        let heard = Rc::new(Cell::new(None));
        let heard_slot = Rc::clone(&heard);
        let mut shell = Shell::new(Rc::new(TestPlatform::new()), move |app: &mut App| {
            app.platform_callbacks_mut().on_view_close_requested =
                Some(Rc::new(move |_app, view| heard_slot.set(Some(view))));
        });
        shell.view_close_requested(ViewId(3));
        assert_eq!(heard.get(), Some(ViewId(3)));
    }

    #[test]
    fn a_window_change_reaches_the_framework() {
        let heard = Rc::new(Cell::new(None));
        let heard_slot = Rc::clone(&heard);
        let mut shell = Shell::new(Rc::new(TestPlatform::new()), move |app: &mut App| {
            app.platform_callbacks_mut().on_window_changed =
                Some(Rc::new(move |_app, view| heard_slot.set(Some(view))));
        });
        shell.window_changed(ViewId(5));
        assert_eq!(heard.get(), Some(ViewId(5)));
    }

    #[test]
    fn a_lifecycle_state_the_host_reports_reaches_the_scheduler_with_the_states_between() {
        let mut shell = Shell::new(Rc::new(TestPlatform::new()), |_app| {});
        shell.app_lifecycle_state_changed(AppLifecycleState::Resumed);
        shell.app_lifecycle_state_changed(AppLifecycleState::Hidden);
        let mut app = shell.app();
        assert_eq!(
            SchedulerBinding::lifecycle_state(&mut app),
            Some(AppLifecycleState::Hidden)
        );
        assert!(!SchedulerBinding::frames_enabled(&mut app));
    }

    #[test]
    fn resuming_marks_every_view_to_be_rendered_and_asks_each_for_a_frame() {
        use inset_rendering::RendererBinding;

        let platform = Rc::new(TestPlatform::new().with_views(&[IMPLICIT_VIEW_ID, ViewId(1)]));
        let first = platform.test_view(IMPLICIT_VIEW_ID);
        let mut shell = Shell::new(platform.clone(), |_app| {});
        let render_view = {
            let mut app = shell.app();
            let binding = RendererBinding::instance(&mut app);
            binding.init_render_view(&mut app, first.clone())
        };
        let at = |ms| Frame {
            elapsed: Duration::from_millis(ms),
        };
        shell.frame(&[IMPLICIT_VIEW_ID], at(16));
        shell.frame(&[IMPLICIT_VIEW_ID], at(32));
        assert_eq!(
            first.presented(),
            1,
            "nothing changed after the first frame"
        );

        shell.app_lifecycle_state_changed(AppLifecycleState::Hidden);
        platform.take_frame_requests();
        shell.app_lifecycle_state_changed(AppLifecycleState::Resumed);
        assert_eq!(
            platform.take_frame_requests(),
            [IMPLICIT_VIEW_ID, ViewId(1)]
        );
        assert!(render_view.needs_composite_frame(&shell.app()));
        shell.frame(&[IMPLICIT_VIEW_ID], at(48));
        assert_eq!(first.presented(), 2, "drawn again though nothing changed");
    }

    #[test]
    fn request_app_exit_is_answered_by_the_services_binding() {
        use inset_embedder::AppExitResponse;
        use inset_foundation::{Handle, Task};
        use inset_services::{ServicesBinding, ServicesBindingOverridesObject};

        #[derive(Default)]
        struct Refusing;

        impl ServicesBindingOverridesObject for Refusing {
            fn handle_request_app_exit(
                self: Handle<Self>,
                _app: &mut App,
            ) -> Task<AppExitResponse> {
                Task::ready(AppExitResponse::Cancel)
            }
        }

        let mut shell = Shell::new(Rc::new(TestPlatform::new()), |_app| {});
        let (reply, answer) = recorded_reply();
        shell.request_app_exit(reply);
        assert_eq!(answer.get(), Some(AppExitResponse::Exit));

        let refusing = shell.app().create(Refusing);
        let binding = ServicesBinding::instance(&mut shell.app());
        binding.set_overrides(&mut shell.app(), Rc::new(refusing));
        let (reply, answer) = recorded_reply();
        shell.request_app_exit(reply);
        assert_eq!(answer.get(), Some(AppExitResponse::Cancel));
    }

    /// The reply is answered inside the call that decides.
    #[test]
    fn request_app_exit_answers_during_the_call_in_which_the_framework_decides() {
        use inset_embedder::AppExitResponse;
        use inset_foundation::{Completer, Handle, Task};
        use inset_services::{ServicesBinding, ServicesBindingOverridesObject};

        struct Deciding {
            decision: Completer<AppExitResponse>,
        }

        impl ServicesBindingOverridesObject for Deciding {
            fn handle_request_app_exit(self: Handle<Self>, app: &mut App) -> Task<AppExitResponse> {
                let decision = app.get(self).decision.future();
                app.spawn(async move |_cx| decision.await)
            }
        }

        let decision = Completer::new();
        let mut shell = Shell::new(Rc::new(TestPlatform::new()), |_app| {});
        let deciding = shell.app().create(Deciding {
            decision: decision.clone(),
        });
        let binding = ServicesBinding::instance(&mut shell.app());
        binding.set_overrides(&mut shell.app(), Rc::new(deciding));

        let (reply, answer) = recorded_reply();
        shell.request_app_exit(reply);
        assert_eq!(answer.get(), None);

        decision.complete(&mut shell.app(), AppExitResponse::Exit);
        assert_eq!(answer.get(), None, "the task resumes at a checkpoint");
        shell.wake(Duration::ZERO);
        assert_eq!(answer.get(), Some(AppExitResponse::Exit));
    }

    #[test]
    fn new_runs_setup_and_frame_runs_both_scheduler_phases() {
        let platform = Rc::new(platform_with_a_view());
        let mut setup_ran = false;
        let mut shell = Shell::new(platform.clone(), |_app: &mut App| {
            setup_ran = true;
        });
        assert!(setup_ran);
        assert!(platform.frame_requests().is_empty());

        SchedulerBinding::schedule_frame(&mut shell.app(), FrameTarget::View(IMPLICIT_VIEW_ID));
        assert_eq!(platform.frame_requests(), [IMPLICIT_VIEW_ID]);
        shell.frame(
            &[IMPLICIT_VIEW_ID],
            Frame {
                elapsed: Duration::from_millis(16),
            },
        );
    }

    #[test]
    fn pointer_data_packet_reaches_gesture_binding() {
        let ran = Rc::new(Cell::new(false));
        let ran_flag = Rc::clone(&ran);
        let platform = Rc::new(platform_with_a_view());
        let mut shell = Shell::new(platform, |app| {
            let binding = GestureBinding::instance(app);
            binding.pointer_router(app).add_route(
                app,
                1,
                PointerRoute::new(move |_app, _event| {
                    ran_flag.set(true);
                }),
                None,
            );
        });

        shell.pointer_data_packet(PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Down,
            kind: PointerDeviceKind::Mouse,
            buttons: 1,
            pointer_identifier: 1,
            ..PointerData::default()
        }]));
        assert!(ran.get());
    }

    #[test]
    fn key_data_reaches_the_hardware_keyboard() {
        let seen = Rc::new(Cell::new(0));
        let seen_flag = Rc::clone(&seen);
        let platform = Rc::new(TestPlatform::new());
        let mut shell = Shell::new(platform, |app| {
            HardwareKeyboard::instance(app).add_handler(
                app,
                Rc::new(move |_app: &mut App, _event: &KeyEvent| {
                    seen_flag.set(seen_flag.get() + 1);
                    true
                }),
            );
        });

        let key_a = KeyData {
            event_type: KeyEventType::Down,
            physical: PhysicalKeyboardKey::KEY_A.usb_hid_usage,
            logical: LogicalKeyboardKey::KEY_A.key_id,
            character: Some("a".to_owned()),
            ..KeyData::default()
        };
        assert!(shell.key_data(key_a), "the handler claimed the event");
        assert_eq!(seen.get(), 1);

        let keyboard = HardwareKeyboard::instance(&mut shell.app());
        assert!(keyboard.is_logical_key_pressed(&shell.app(), LogicalKeyboardKey::KEY_A));
    }

    #[test]
    fn wake_and_frame_advance_the_app_clock_and_fire_due_timers() {
        let platform = Rc::new(TestPlatform::new());
        let fired = Rc::new(Cell::new(Vec::new()));
        let mut shell = Shell::new(platform, |app| {
            for (name, delay) in [("early", 10), ("late", 30)] {
                let fired = Rc::clone(&fired);
                inset_foundation::Timer::new(
                    app,
                    Duration::from_millis(delay),
                    inset_foundation::Listener::new(move |_app| {
                        let mut log = fired.take();
                        log.push(name);
                        fired.set(log);
                    }),
                );
            }
        });
        assert!(fired.take().is_empty());

        shell.wake(Duration::from_millis(15));
        assert_eq!(fired.take(), vec!["early"]);

        shell.frame(
            &[IMPLICIT_VIEW_ID],
            Frame {
                elapsed: Duration::from_millis(32),
            },
        );
        assert_eq!(fired.take(), vec!["late"]);
    }

    #[test]
    fn a_view_coming_or_going_and_a_change_to_the_screens_are_metrics_changes() {
        use inset_foundation::Handle;
        use inset_rendering::{RendererBinding, RendererBindingOverridesObject};

        struct CountingMetricsChanges {
            count: Rc<Cell<usize>>,
        }

        impl RendererBindingOverridesObject for CountingMetricsChanges {
            fn will_draw_frame(self: Handle<Self>, _app: &mut App) {}

            fn did_draw_frame(self: Handle<Self>, _app: &mut App) {}

            fn did_handle_metrics_changed(self: Handle<Self>, app: &mut App) {
                let count = &app.get(self).count;
                count.set(count.get() + 1);
            }
        }

        let count = Rc::new(Cell::new(0));
        let mut shell = Shell::new(Rc::new(platform_with_a_view()), |_app| {});
        let counting = shell.app().create(CountingMetricsChanges {
            count: Rc::clone(&count),
        });
        let binding = RendererBinding::instance(&mut shell.app());
        binding.set_overrides(&mut shell.app(), Rc::new(counting));

        shell.view_added(ViewId(1));
        assert_eq!(count.get(), 1, "a view added");
        shell.view_removed(ViewId(1));
        assert_eq!(count.get(), 2, "a view removed");
        shell.screens_changed();
        assert_eq!(count.get(), 3, "the screens changed");
    }

    #[test]
    fn view_focus_is_delivered_through_the_dispatcher_callback() {
        use inset_embedder::{ViewFocusDirection, ViewFocusEvent, ViewFocusState};
        let received = Rc::new(Cell::new(None));
        let sink = received.clone();
        let platform = Rc::new(platform_with_a_view());
        let mut shell = Shell::new(platform, move |app| {
            app.platform_callbacks_mut().on_view_focus_change = Some(Rc::new(move |app, event| {
                let sink = sink.clone();
                app.schedule_microtask(inset_foundation::Listener::new(move |_| {
                    sink.set(Some(event))
                }));
            }));
        });
        shell.view_focus_changed(ViewFocusEvent {
            view_id: IMPLICIT_VIEW_ID,
            state: ViewFocusState::Focused,
            direction: ViewFocusDirection::Backward,
        });
        let event = received
            .get()
            .expect("the shell checkpoints the callback's microtask");
        assert_eq!(event.view_id, IMPLICIT_VIEW_ID);
        assert_eq!(event.state, ViewFocusState::Focused);
        assert_eq!(event.direction, ViewFocusDirection::Backward);
    }
}

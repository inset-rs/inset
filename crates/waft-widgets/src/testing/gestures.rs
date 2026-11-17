//! Input a [`WidgetTester`] sends the app: pointers, as Dart's `TestGesture`, and keys.

use std::time::Duration;

use super::{RenderTarget, WidgetTester};

/// One pointer a [`WidgetTester`] drives, as Dart's `TestGesture`: its device, buttons and
/// where it is, in logical coordinates.
#[derive(Clone, Copy, Debug)]
pub struct TestPointer {
    /// The pointer's device and identifier.
    pub id: i64,
    /// What kind of device it is.
    pub kind: waft_embedder::PointerDeviceKind,
    /// The buttons it presses while down.
    pub buttons: i64,
    /// Where it is, in logical coordinates.
    pub position: waft_embedder::Offset,
}

impl TestPointer {
    /// A touch pointer with the primary button, as `tester.startGesture` makes.
    pub fn touch(id: i64) -> TestPointer {
        TestPointer {
            id,
            kind: waft_embedder::PointerDeviceKind::Touch,
            buttons: waft_gestures::K_PRIMARY_BUTTON,
            position: waft_embedder::Offset::ZERO,
        }
    }

    /// A mouse pointer pressing `buttons`, as `tester.createGesture(kind: mouse)` makes.
    pub fn mouse(id: i64, buttons: i64) -> TestPointer {
        TestPointer {
            id,
            kind: waft_embedder::PointerDeviceKind::Mouse,
            buttons,
            position: waft_embedder::Offset::ZERO,
        }
    }
}

impl WidgetTester {
    /// Sends one packet of `pointer` at its position, in logical coordinates, stamped
    /// `Duration.zero` as Dart's `TestPointer` stamps an event it is given no time stamp for.
    pub fn send(&mut self, pointer: &TestPointer, change: waft_embedder::PointerChange) {
        self.send_at(pointer, change, waft_embedder::Offset::ZERO, Duration::ZERO);
    }

    /// [`send`](Self::send) with the packet's `delta` from the pointer's last position, in
    /// logical coordinates, and stamped `time_stamp`, as Dart's `TestPointer` gives its events.
    pub fn send_at(
        &mut self,
        pointer: &TestPointer,
        change: waft_embedder::PointerChange,
        delta: waft_embedder::Offset,
        time_stamp: Duration,
    ) {
        use waft_embedder::{PointerChange, PointerData, PointerDataPacket};
        let buttons = match change {
            PointerChange::Down | PointerChange::Move => pointer.buttons,
            _ => 0,
        };
        let ratio = self.device_pixel_ratio();
        let mut app = self.cell.borrow_mut();
        waft_gestures::GestureBinding::instance(&mut app).handle_pointer_data_packet(
            &mut app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: pointer.kind,
                time_stamp,
                device: pointer.id,
                pointer_identifier: pointer.id,
                buttons,
                physical_x: pointer.position.dx() * ratio,
                physical_y: pointer.position.dy() * ratio,
                physical_delta_x: delta.dx() * ratio,
                physical_delta_y: delta.dy() * ratio,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
        drop(app);
        self.cell.checkpoint();
    }

    /// Dart's `startGesture(at)`: a pointer down at `at`.
    pub fn down(&mut self, pointer: &mut TestPointer, at: waft_embedder::Offset) {
        pointer.position = at;
        if pointer.kind == waft_embedder::PointerDeviceKind::Mouse {
            self.send(pointer, waft_embedder::PointerChange::Hover);
        }
        self.send(pointer, waft_embedder::PointerChange::Down);
    }

    /// Dart's `gesture.moveTo(at)`: a move while down, a hover while up, stamped `Duration.zero`.
    pub fn move_to(&mut self, pointer: &mut TestPointer, at: waft_embedder::Offset, down: bool) {
        let delta = at - pointer.position;
        pointer.position = at;
        let change = if down {
            waft_embedder::PointerChange::Move
        } else {
            waft_embedder::PointerChange::Hover
        };
        self.send_at(pointer, change, delta, Duration::ZERO);
    }

    /// Dart's `gesture.up()`.
    pub fn up(&mut self, pointer: &TestPointer) {
        self.send(pointer, waft_embedder::PointerChange::Up);
    }

    /// Dart's `gesture.cancel()`.
    pub fn cancel(&mut self, pointer: &TestPointer) {
        self.send(pointer, waft_embedder::PointerChange::Cancel);
    }

    /// Dart's `gesture.removePointer()`: a `PointerRemovedEvent` with the pointer's id handed
    /// to the gesture binding, as `TestGesture` hands it, so a recognizer tracking the pointer
    /// sees it even while the pointer is down.
    pub fn remove_pointer(&mut self, pointer: &TestPointer) {
        use waft_gestures::{GestureBinding, PointerEvent, PointerRemovedEvent};
        let mut app = self.cell.borrow_mut();
        GestureBinding::instance(&mut app).handle_pointer_event(
            &mut app,
            PointerEvent::Removed(PointerRemovedEvent {
                kind: pointer.kind,
                device: pointer.id,
                pointer: pointer.id,
                position: pointer.position,
                ..PointerRemovedEvent::default()
            }),
        );
        app.drain_microtasks();
        drop(app);
        self.cell.checkpoint();
    }

    /// Dart's `gesture.addPointer()` for a mouse, at `at`.
    pub fn add_pointer(&mut self, pointer: &mut TestPointer, at: waft_embedder::Offset) {
        pointer.position = at;
        self.send(pointer, waft_embedder::PointerChange::Add);
    }

    /// Dart's `tester.sendKeyEvent(key)`: a key down and a key up of `physical` and `logical`.
    pub fn send_key_event(
        &mut self,
        physical: waft_services::PhysicalKeyboardKey,
        logical: waft_services::LogicalKeyboardKey,
    ) {
        use waft_embedder::KeyEventType;
        let mut app = self.cell.borrow_mut();
        for event_type in [KeyEventType::Down, KeyEventType::Up] {
            handle_key_event(&mut app, event_type, physical, logical, None);
        }
        app.drain_microtasks();
        drop(app);
        self.cell.checkpoint();
    }

    /// Dart's `tester.sendKeyDownEvent(key)`: a key down of `physical` and `logical`.
    pub fn send_key_down_event(
        &mut self,
        physical: waft_services::PhysicalKeyboardKey,
        logical: waft_services::LogicalKeyboardKey,
    ) {
        self.send_key_event_of(waft_embedder::KeyEventType::Down, physical, logical, None);
    }

    /// Dart's `tester.sendKeyDownEvent(key, character: character)`: a key down that carries
    /// the character the platform typed.
    pub fn send_key_down_event_with_character(
        &mut self,
        physical: waft_services::PhysicalKeyboardKey,
        logical: waft_services::LogicalKeyboardKey,
        character: &str,
    ) {
        self.send_key_event_of(
            waft_embedder::KeyEventType::Down,
            physical,
            logical,
            Some(character),
        );
    }

    /// Dart's `tester.sendKeyUpEvent(key)`: a key up of `physical` and `logical`.
    pub fn send_key_up_event(
        &mut self,
        physical: waft_services::PhysicalKeyboardKey,
        logical: waft_services::LogicalKeyboardKey,
    ) {
        self.send_key_event_of(waft_embedder::KeyEventType::Up, physical, logical, None);
    }

    /// One key event, then the microtasks it scheduled and the checkpoint after them.
    fn send_key_event_of(
        &mut self,
        event_type: waft_embedder::KeyEventType,
        physical: waft_services::PhysicalKeyboardKey,
        logical: waft_services::LogicalKeyboardKey,
        character: Option<&str>,
    ) {
        let mut app = self.cell.borrow_mut();
        handle_key_event(&mut app, event_type, physical, logical, character);
        app.drain_microtasks();
        drop(app);
        self.cell.checkpoint();
    }

    /// Dart's `tester.tap(finder)`: a tap at the centre of the one widget `target` finds,
    /// with the next pointer.
    pub fn tap(&mut self, target: impl RenderTarget) {
        let at = self.center_for(target, "tap");
        let pointer = self.next_pointer();
        self.tap_at(at, pointer);
    }

    /// Dart's `tester.tapAt(at)`: down and up with no frame between.
    pub fn tap_at(&mut self, at: waft_embedder::Offset, pointer: i64) {
        let mut touch = TestPointer::touch(pointer);
        self.down(&mut touch, at);
        self.up(&touch);
    }

    /// Dart's `gesture.moveBy(offset)`: a move by `by` from where the pointer is, while down,
    /// or a hover while up.
    pub fn move_by(&mut self, pointer: &mut TestPointer, by: waft_embedder::Offset, down: bool) {
        let at = pointer.position + by;
        self.move_to(pointer, at, down);
    }

    /// Dart's `tester.fling(finder, offset, speed)`: [`fling_from`](Self::fling_from) the
    /// centre of the one widget `target` finds, with the next pointer.
    pub fn fling(&mut self, target: impl RenderTarget, offset: waft_embedder::Offset, speed: f64) {
        let start = self.center_for(target, "fling");
        let pointer = self.next_pointer();
        self.fling_from(start, offset, speed, pointer);
    }

    /// Dart's `tester.flingFrom(start, offset, speed)`: a touch down at `start`, 51 moves to
    /// `start + offset` stamped from zero at `speed` logical pixels a second, a frame after
    /// every 16ms of stamps, and an up.
    pub fn fling_from(
        &mut self,
        start: waft_embedder::Offset,
        offset: waft_embedder::Offset,
        speed: f64,
        pointer: i64,
    ) {
        self.fling_from_after(
            start,
            offset,
            speed,
            pointer,
            waft_embedder::Offset::ZERO,
            Duration::from_secs(1),
        );
    }

    /// Dart's `tester.flingFrom(start, offset, speed, initialOffset:, initialOffsetDelay:)`: the
    /// touch first moves by `initial_offset` and waits `initial_offset_delay`, a frame of it,
    /// before the fling's moves.
    pub fn fling_from_after(
        &mut self,
        start: waft_embedder::Offset,
        offset: waft_embedder::Offset,
        speed: f64,
        pointer: i64,
        initial_offset: waft_embedder::Offset,
        initial_offset_delay: Duration,
    ) {
        use waft_embedder::{Offset, PointerChange};
        debug_assert!(offset.distance() > 0.0);
        debug_assert!(speed > 0.0); // speed is pixels/second
        let micros = |time_stamp: f64| Duration::from_micros(time_stamp as u64);
        let mut touch = TestPointer::touch(pointer);
        const K_MOVE_COUNT: u32 = 50; // Needs to be >= kHistorySize, see _LeastSquaresVelocityTrackerStrategy
        let frame_interval = Duration::from_millis(16).as_micros() as f64;
        let time_stamp_delta = 1000000.0 * offset.distance() / (K_MOVE_COUNT as f64 * speed);
        let mut time_stamp = 0.0_f64;
        let mut last_time_stamp = time_stamp;
        touch.position = start;
        self.send_at(
            &touch,
            PointerChange::Down,
            Offset::ZERO,
            micros(time_stamp.round()),
        );
        if initial_offset.distance() > 0.0 {
            touch.position = start + initial_offset;
            self.send_at(
                &touch,
                PointerChange::Move,
                initial_offset,
                micros(time_stamp.round()),
            );
            time_stamp += initial_offset_delay.as_micros() as f64;
            self.pump_by(initial_offset_delay);
        }
        for i in 0..=K_MOVE_COUNT {
            let location = start
                + initial_offset
                + Offset::lerp(
                    Some(Offset::ZERO),
                    Some(offset),
                    f64::from(i) / f64::from(K_MOVE_COUNT),
                )
                .expect("both ends are given");
            let delta = location - touch.position;
            touch.position = location;
            self.send_at(
                &touch,
                PointerChange::Move,
                delta,
                micros(time_stamp.round()),
            );
            time_stamp += time_stamp_delta;
            if time_stamp - last_time_stamp > frame_interval {
                self.pump_by(micros((time_stamp - last_time_stamp).trunc()));
                last_time_stamp = time_stamp;
            }
        }
        self.send_at(
            &touch,
            PointerChange::Up,
            Offset::ZERO,
            micros(time_stamp.round()),
        );
    }

    /// Dart's `tester.drag(finder, offset)`: [`drag_from`](Self::drag_from) the centre of the
    /// one widget `target` finds, with the next pointer.
    pub fn drag(&mut self, target: impl RenderTarget, offset: waft_embedder::Offset) {
        let start = self.center_for(target, "drag");
        let pointer = self.next_pointer();
        self.drag_from(start, offset, pointer);
    }

    /// Dart's `tester.dragFrom(start, offset)`: a touch down at `start`, moves that cross
    /// flutter_test's 20 pixel drag slop box in separate steps, and an up, with no frame
    /// between.
    pub fn drag_from(
        &mut self,
        start: waft_embedder::Offset,
        offset: waft_embedder::Offset,
        pointer: i64,
    ) {
        use waft_embedder::Offset;
        const K_DRAG_SLOP_DEFAULT: f64 = 20.0;
        let touch_slop_x = K_DRAG_SLOP_DEFAULT;
        let touch_slop_y = K_DRAG_SLOP_DEFAULT;
        let mut gesture = TestPointer::touch(pointer);
        self.down(&mut gesture, start);
        let mut move_by = |tester: &mut WidgetTester, by: Offset| {
            let at = gesture.position + by;
            tester.move_to(&mut gesture, at, true);
        };

        let x_sign = dart_sign(offset.dx());
        let y_sign = dart_sign(offset.dy());

        let offset_x = offset.dx();
        let offset_y = offset.dy();

        let separate_x = offset.dx().abs() > touch_slop_x && touch_slop_x > 0.0;
        let separate_y = offset.dy().abs() > touch_slop_y && touch_slop_y > 0.0;

        if separate_y || separate_x {
            let offset_slope = offset_y / offset_x;
            let inverse_offset_slope = offset_x / offset_y;
            let slop_slope = touch_slop_y / touch_slop_x;
            let absolute_offset_slope = offset_slope.abs();
            let signed_slop_x = touch_slop_x * x_sign;
            let signed_slop_y = touch_slop_y * y_sign;
            if absolute_offset_slope != slop_slope {
                // The drag goes through one or both of the extents of the edges of the box.
                if absolute_offset_slope < slop_slope {
                    debug_assert!(offset_x.abs() > touch_slop_x);
                    // The drag goes through the vertical edge of the box.
                    // It is guaranteed that the |offsetX| > touchSlopX.
                    let diff_y = offset_slope.abs() * touch_slop_x * y_sign;

                    // The vector from the origin to the vertical edge.
                    move_by(self, Offset::new(signed_slop_x, diff_y));
                    if offset_y.abs() <= touch_slop_y {
                        // The drag ends on or before getting to the horizontal extension of the horizontal edge.
                        move_by(
                            self,
                            Offset::new(offset_x - signed_slop_x, offset_y - diff_y),
                        );
                    } else {
                        let diff_y2 = signed_slop_y - diff_y;
                        let diff_x2 = inverse_offset_slope * diff_y2;

                        // The vector from the edge of the box to the horizontal extension of the horizontal edge.
                        move_by(self, Offset::new(diff_x2, diff_y2));
                        move_by(
                            self,
                            Offset::new(
                                offset_x - diff_x2 - signed_slop_x,
                                offset_y - signed_slop_y,
                            ),
                        );
                    }
                } else {
                    debug_assert!(offset_y.abs() > touch_slop_y);
                    // The drag goes through the horizontal edge of the box.
                    // It is guaranteed that the |offsetY| > touchSlopY.
                    let diff_x = inverse_offset_slope.abs() * touch_slop_y * x_sign;

                    // The vector from the origin to the vertical edge.
                    move_by(self, Offset::new(diff_x, signed_slop_y));
                    if offset_x.abs() <= touch_slop_x {
                        // The drag ends on or before getting to the vertical extension of the vertical edge.
                        move_by(
                            self,
                            Offset::new(offset_x - diff_x, offset_y - signed_slop_y),
                        );
                    } else {
                        let diff_x2 = signed_slop_x - diff_x;
                        let diff_y2 = offset_slope * diff_x2;

                        // The vector from the edge of the box to the vertical extension of the vertical edge.
                        move_by(self, Offset::new(diff_x2, diff_y2));
                        move_by(
                            self,
                            Offset::new(
                                offset_x - signed_slop_x,
                                offset_y - diff_y2 - signed_slop_y,
                            ),
                        );
                    }
                }
            } else {
                // The drag goes through the corner of the box.
                move_by(self, Offset::new(signed_slop_x, signed_slop_y));
                move_by(
                    self,
                    Offset::new(offset_x - signed_slop_x, offset_y - signed_slop_y),
                );
            }
        } else {
            // The drag ends inside the box.
            move_by(self, offset);
        }
        self.up(&gesture);
    }

    /// Dart's `tester.longPress(finder)`: a long press at the centre of the one widget
    /// `target` finds, with the next pointer.
    pub fn long_press(&mut self, target: impl RenderTarget) {
        let at = self.center_for(target, "longPress");
        let pointer = self.next_pointer();
        self.long_press_at(at, pointer);
    }

    /// Dart's `tester.longPressAt(at)`: down, a `kLongPressTimeout + kPressTimeout` wait, and up.
    pub fn long_press_at(&mut self, at: waft_embedder::Offset, pointer: i64) {
        let mut touch = TestPointer::touch(pointer);
        self.down(&mut touch, at);
        self.pump_by(waft_gestures::K_LONG_PRESS_TIMEOUT + waft_gestures::K_PRESS_TIMEOUT);
        self.up(&touch);
    }
}

/// Dart's `double.sign`: -1.0, 0.0 or 1.0, where Rust's `signum` answers 1.0 for zero.
fn dart_sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// Hands one key event to the framework, as the shell hands one a platform reports.
fn handle_key_event(
    app: &mut waft_foundation::App,
    event_type: waft_embedder::KeyEventType,
    physical: waft_services::PhysicalKeyboardKey,
    logical: waft_services::LogicalKeyboardKey,
    character: Option<&str>,
) {
    use waft_embedder::{KeyData, KeyEventDeviceType};
    waft_services::KeyEventManager::instance(app).handle_key_data(
        app,
        KeyData {
            time_stamp: std::time::Duration::ZERO,
            event_type,
            device_type: KeyEventDeviceType::Keyboard,
            physical: physical.usb_hid_usage,
            logical: logical.key_id,
            character: character.map(str::to_owned),
            synthesized: false,
        },
    );
}

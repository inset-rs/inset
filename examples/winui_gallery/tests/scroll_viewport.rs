//! Kit-native scrolling remains virtualized, reports dimensions, and accepts native input.
#![feature(arbitrary_self_types)]

use inset_embedder::{
    Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind,
};
use inset_foundation::{Handle, Listenable, Listener};
use inset_gestures::GestureBinding;
use inset_painting::{Axis, EdgeInsetsGeometry};
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Default)]
struct Probe {
    controller: Rc<Cell<Option<Handle<ScrollViewportController>>>>,
    built: Rc<Cell<usize>>,
    metrics: Rc<RefCell<Vec<ScrollViewportMetrics>>>,
}

fn virtualized(axis: Axis) -> (Fixture, Probe) {
    let probe = Probe::default();
    let output = probe.clone();
    let f = Fixture::with_root([320, 200], move |app| {
        winui_gallery::install_fonts(app);
        let controller = ScrollViewportController::new(app);
        output.controller.set(Some(controller));
        let metrics = output.metrics.clone();
        controller.add_listener(
            app,
            Listener::new(move |app| metrics.borrow_mut().push(controller.metrics(app))),
        );
        let native = controller.native_controller(app);
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    let built = output.built.clone();
                    let list = ListView::builder(move |_, _, index| {
                        built.set(built.get() + 1);
                        Some(
                            Button::text(format!("Item {index}"), Listener::new(|_| {}))
                                .into_widget(),
                        )
                    })
                    .item_count(1000)
                    .build()
                    .item_extent(50.0)
                    .scroll_direction(axis)
                    .controller(native)
                    .padding(EdgeInsetsGeometry::ZERO);
                    FocusScope::new(ThemeScope::new(
                        Theme::Light,
                        ScrollMetricsObserver::new(controller, list),
                    ))
                    .autofocus(true)
                    .into_widget()
                })
                .into_widget(),
        );
    });
    (f, probe)
}

fn settle(f: &mut Fixture) {
    for _ in 0..3 {
        f.pump();
    }
}

fn wheel(f: &mut Fixture, x: f64, y: f64) {
    let mut app = f.cell.borrow_mut();
    GestureBinding::instance(&mut app).handle_pointer_data_packet(
        &mut app,
        PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Hover,
            kind: PointerDeviceKind::Mouse,
            signal_kind: Some(PointerSignalKind::Scroll),
            physical_x: 100.0,
            physical_y: 100.0,
            scroll_delta_x: x,
            scroll_delta_y: y,
            ..Default::default()
        }]),
    );
    drop(app);
    f.cell.checkpoint();
    settle(f);
}

#[test]
fn controller_commands_clamp_and_keep_the_native_list_virtualized() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let (mut f, p) = virtualized(axis);
        settle(&mut f);
        let controller = p.controller.get().unwrap();
        let m = controller.metrics(&f.cell.borrow());
        let viewport = if axis == Axis::Horizontal {
            320.0
        } else {
            200.0
        };
        assert_eq!(m.viewport_length, viewport);
        assert_eq!(m.extent_length, 50_000.0);
        assert!(p.built.get() < 30, "eagerly built {} items", p.built.get());
        controller.change_view(&mut f.cell.borrow_mut(), 500.0, true);
        settle(&mut f);
        assert_eq!(controller.metrics(&f.cell.borrow()).offset, 500.0);
        assert!(p.built.get() < 60);
        controller.change_view(&mut f.cell.borrow_mut(), 1_000_000.0, true);
        settle(&mut f);
        let m = controller.metrics(&f.cell.borrow());
        assert_eq!(m.offset, m.scrollable_length);
        assert!(
            p.built.get() < 90,
            "jumping to the end materialized all items"
        );
        controller.change_view(&mut f.cell.borrow_mut(), -30.0, true);
        settle(&mut f);
        assert_eq!(controller.metrics(&f.cell.borrow()).offset, 0.0);
        assert!(p.metrics.borrow().iter().any(|m| m.offset == 500.0));
        assert!(
            p.metrics
                .borrow()
                .iter()
                .all(|m| m.viewport_length == viewport)
        );
    }
}

#[test]
fn native_mouse_wheel_and_keyboard_scroll_the_same_controller() {
    let (mut f, p) = virtualized(Axis::Vertical);
    let controller = p.controller.get().unwrap();
    wheel(&mut f, 0.0, 100.0);
    assert_eq!(controller.metrics(&f.cell.borrow()).offset, 100.0);
    controller.change_view(&mut f.cell.borrow_mut(), 0.0, true);
    settle(&mut f);
    f.focus("Item 0");
    for event in [
        KeyEvent::Down(KeyDownEvent::new(
            PhysicalKeyboardKey::PAGE_DOWN,
            LogicalKeyboardKey::PAGE_DOWN,
            f.at,
        )),
        KeyEvent::Up(KeyUpEvent::new(
            PhysicalKeyboardKey::PAGE_DOWN,
            LogicalKeyboardKey::PAGE_DOWN,
            f.at,
        )),
    ] {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    }
    settle(&mut f);
    assert!(controller.metrics(&f.cell.borrow()).offset > 0.0);
    let (mut horizontal, hp) = virtualized(Axis::Horizontal);
    wheel(&mut horizontal, 90.0, 0.0);
    assert_eq!(
        hp.controller
            .get()
            .unwrap()
            .metrics(&horizontal.cell.borrow())
            .offset,
        90.0
    );
}

#[test]
fn simple_viewport_receives_native_touch_drag_and_reports_its_content_extent() {
    let controller_slot = Rc::new(Cell::new(None));
    let slot = controller_slot.clone();
    let mut f = Fixture::with_root([320, 200], move |app| {
        let controller = ScrollViewportController::new(app);
        slot.set(Some(controller));
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    ScrollViewport::new(
                        Axis::Vertical,
                        controller,
                        SizedBox::new()
                            .height(1000.0)
                            .child(Text::new("Scrollable content")),
                    )
                    .into_widget()
                })
                .into_widget(),
        );
    });
    let controller = controller_slot.get().unwrap();
    assert_eq!(controller.metrics(&f.cell.borrow()).extent_length, 1000.0);
    f.send(PointerChange::Down, Offset::new(100.0, 150.0));
    for (y, dy) in [(110.0, -40.0), (50.0, -60.0)] {
        f.at += std::time::Duration::from_millis(20);
        let mut app = f.cell.borrow_mut();
        GestureBinding::instance(&mut app).handle_pointer_data_packet(
            &mut app,
            PointerDataPacket::new(vec![PointerData {
                change: PointerChange::Move,
                kind: PointerDeviceKind::Touch,
                time_stamp: f.at,
                pointer_identifier: f.touch_pointer,
                physical_x: 100.0,
                physical_y: y,
                physical_delta_y: dy,
                ..Default::default()
            }]),
        );
        drop(app);
        f.cell.checkpoint();
    }
    f.at += std::time::Duration::from_millis(20);
    f.send(PointerChange::Up, Offset::new(100.0, 50.0));
    settle(&mut f);
    assert!(controller.metrics(&f.cell.borrow()).offset > 0.0);
}

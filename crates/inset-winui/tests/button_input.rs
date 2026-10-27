#![feature(arbitrary_self_types)]

use inset_embedder::*;
use inset_foundation::{AppCell, Listener};
use inset_gestures::GestureBinding;
use inset_scheduler::SchedulerBinding;
use inset_services::*;
use inset_test::{IMPLICIT_VIEW_ID, TestPlatform, TestView};
use inset_widgets::*;
use inset_winui::{CommonState, CommonStates, ControlStates};
use std::{cell::Cell, rc::Rc, time::Duration};

/// An 800x600 view on a macOS host.
fn host() -> TestPlatform {
    TestPlatform::new()
        .on(TargetPlatform::MacOS)
        .with_view(Rc::new(TestView::new(800.0, 600.0)))
}

struct Fixture {
    cell: Rc<AppCell>,
    clicks: Rc<Cell<usize>>,
    states: Rc<Cell<ControlStates>>,
    accepts_return: bool,
    time: Duration,
}
impl Fixture {
    fn new(accepts_return: bool) -> Self {
        let mut fixture = Self {
            cell: AppCell::with_platform(Rc::new(host())),
            clicks: Rc::new(Cell::new(0)),
            states: Rc::new(Cell::new(ControlStates::default())),
            accepts_return,
            time: Duration::ZERO,
        };
        fixture.mount(true);
        fixture
    }
    fn mount(&mut self, enabled: bool) {
        let clicks = self.clicks.clone();
        let states = self.states.clone();
        {
            let mut app = self.cell.borrow_mut();
            let root = Shortcuts::new(
                WidgetsApp::default_shortcuts(&app),
                Center::new().child(
                    SizedBox::new().width(200.0).height(100.0).child(
                        CommonStates::new(
                            Listener::new(move |_| clicks.set(clicks.get() + 1)),
                            move |_, _, value| {
                                states.set(value);
                                SizedBox::expand().into_widget()
                            },
                        )
                        .accepts_return(self.accepts_return)
                        .is_enabled(enabled),
                    ),
                ),
            )
            .into_widget();
            run_app(&mut app, root);
        }
        self.cell.elapse(Duration::ZERO);
        self.pump();
    }
    fn pump(&mut self) {
        self.time += Duration::from_millis(20);
        let mut app = self.cell.borrow_mut();
        let views = [IMPLICIT_VIEW_ID];
        SchedulerBinding::handle_begin_frame(&mut app, &views, Some(self.time));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(&mut app, &views);
        app.drain_microtasks();
    }
    fn node(&self) -> AnyFocusNode {
        let mut app = self.cell.borrow_mut();
        let root = FocusManager::instance(&mut app).root_scope(&app).as_node();
        root.traversal_descendants(&mut app)[0]
    }
    fn focus(&mut self) {
        let node = self.node();
        node.request_focus(&mut self.cell.borrow_mut(), None);
        self.pump();
    }
    fn key(&mut self, kind: usize, key: LogicalKeyboardKey) {
        let physical = match key {
            LogicalKeyboardKey::SPACE => PhysicalKeyboardKey::SPACE,
            LogicalKeyboardKey::ENTER => PhysicalKeyboardKey::ENTER,
            _ => PhysicalKeyboardKey::KEY_A,
        };
        let event = match kind {
            0 => KeyEvent::Down(KeyDownEvent::new(physical, key, self.time)),
            1 => KeyEvent::Repeat(KeyRepeatEvent::new(physical, key, self.time)),
            _ => KeyEvent::Up(KeyUpEvent::new(physical, key, self.time)),
        };
        {
            let mut app = self.cell.borrow_mut();
            HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
        }
        self.pump();
    }
}

#[test]
fn space_and_enter_press_visually_then_click_once_on_release() {
    for key in [LogicalKeyboardKey::SPACE, LogicalKeyboardKey::ENTER] {
        let mut fixture = Fixture::new(true);
        fixture.focus();
        fixture.key(0, key);
        assert_eq!(fixture.clicks.get(), 0);
        assert_eq!(fixture.states.get().common, CommonState::Pressed);
        fixture.key(1, key);
        assert_eq!(fixture.clicks.get(), 0);
        fixture.key(2, key);
        assert_eq!(fixture.clicks.get(), 1);
        assert_ne!(fixture.states.get().common, CommonState::Pressed);
    }
}

#[test]
fn another_key_cancels_a_keyboard_press() {
    let mut fixture = Fixture::new(true);
    fixture.focus();
    fixture.key(0, LogicalKeyboardKey::SPACE);
    fixture.key(0, LogicalKeyboardKey::KEY_A);
    assert_ne!(fixture.states.get().common, CommonState::Pressed);
    fixture.key(2, LogicalKeyboardKey::KEY_A);
    fixture.key(2, LogicalKeyboardKey::SPACE);
    assert_eq!(fixture.clicks.get(), 0);
}

#[test]
fn controls_that_reject_return_do_not_use_the_app_activation_shortcut() {
    let mut fixture = Fixture::new(false);
    fixture.focus();
    fixture.key(0, LogicalKeyboardKey::ENTER);
    fixture.key(1, LogicalKeyboardKey::ENTER);
    fixture.key(2, LogicalKeyboardKey::ENTER);
    assert_eq!(fixture.clicks.get(), 0);
    assert_ne!(fixture.states.get().common, CommonState::Pressed);
}

#[test]
fn disabling_a_held_button_cancels_its_pending_click() {
    let mut fixture = Fixture::new(true);
    fixture.focus();
    fixture.key(0, LogicalKeyboardKey::SPACE);
    fixture.mount(false);
    assert_eq!(fixture.states.get().common, CommonState::Disabled);
    fixture.key(2, LogicalKeyboardKey::SPACE);
    fixture.mount(true);
    fixture.focus();
    assert_eq!(fixture.clicks.get(), 0);
    assert_ne!(fixture.states.get().common, CommonState::Pressed);
}

#[test]
fn losing_focus_cancels_a_held_button() {
    let mut fixture = Fixture::new(true);
    fixture.focus();
    fixture.key(0, LogicalKeyboardKey::SPACE);
    {
        let mut app = fixture.cell.borrow_mut();
        let focused = FocusManager::instance(&mut app)
            .primary_focus(&app)
            .unwrap();
        focused.unfocus(&mut app, UnfocusDisposition::Scope);
    }
    fixture.pump();
    fixture.key(2, LogicalKeyboardKey::SPACE);
    fixture.focus();
    assert_eq!(fixture.clicks.get(), 0);
    assert_ne!(fixture.states.get().common, CommonState::Pressed);
}

#[test]
fn a_mouse_click_leaves_focus_for_keyboard_navigation() {
    let mut fixture = Fixture::new(true);
    for change in [PointerChange::Add, PointerChange::Down, PointerChange::Up] {
        let mut app = fixture.cell.borrow_mut();
        GestureBinding::instance(&mut app).handle_pointer_data_packet(
            &mut app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Mouse,
                pointer_identifier: 7,
                physical_x: 400.0,
                physical_y: 300.0,
                buttons: if change == PointerChange::Down { 1 } else { 0 },
                ..Default::default()
            }]),
        );
    }
    fixture.pump();
    {
        let mut app = fixture.cell.borrow_mut();
        assert!(
            FocusManager::instance(&mut app)
                .primary_focus(&app)
                .is_none()
        );
    }
    assert_eq!(fixture.clicks.get(), 1);
    assert!(!fixture.states.get().focused);
    fixture.focus();
    fixture.key(0, LogicalKeyboardKey::SPACE);
    fixture.key(2, LogicalKeyboardKey::SPACE);
    assert_eq!(fixture.clicks.get(), 2);
}

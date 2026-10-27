//! Native drag/drop prerequisites exercised through real hit testing and pointer routing.
#![feature(arbitrary_self_types)]

use inset_embedder::{Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::GestureBinding;
use inset_widgets::*;
use inset_winui_test_support::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Debug, Default)]
struct Probe {
    state: Rc<Cell<Option<Handle<PageState>>>>,
    events: Rc<RefCell<Vec<String>>>,
}

#[derive(Debug)]
struct Page(Probe, bool);

struct PageState {
    state: StateData<Page>,
    show_source: bool,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            show_source: true,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let probe = self.widget(app).0.clone();
        probe.state.set(Some(self));
        let events = probe.events.clone();
        let accepted = probe.events.clone();
        let canceled = probe.events.clone();
        let completed = probe.events.clone();
        let mut children = vec![
            Positioned::new(
                DragTarget::<u32>::new(|_, _, candidates, _| {
                    SizedBox::new()
                        .width(100.0)
                        .height(80.0)
                        .child(Text::new(if candidates.is_empty() {
                            "Target"
                        } else {
                            "Candidate"
                        }))
                        .into_widget()
                })
                .on_accept(Rc::new(move |_, data| {
                    accepted.borrow_mut().push(format!("accept {data}"))
                })),
            )
            .left(200.0)
            .top(30.0)
            .into_widget(),
        ];
        if app.get(self).show_source {
            let source = Draggable::new(
                SizedBox::new()
                    .width(80.0)
                    .height(60.0)
                    .child(Text::new("Source")),
                Text::new("Feedback"),
            )
            .data(42u32)
            .on_drag_started(Listener::new(move |_| {
                events.borrow_mut().push("start".into())
            }))
            .on_drag_completed(Listener::new(move |_| {
                completed.borrow_mut().push("complete".into())
            }))
            .on_draggable_canceled(Rc::new(move |_, _, _| {
                canceled.borrow_mut().push("cancel".into())
            }));
            let source = if self.widget(app).1 {
                LongPressDraggable {
                    draggable: source,
                    haptic_feedback_on_start: false,
                    delay: std::time::Duration::from_millis(500),
                }
                .into_widget()
            } else {
                source.into_widget()
            };
            children.push(Positioned::new(source).left(20.0).top(30.0).into_widget());
        }
        Stack::new().children(children).into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    fixture_with_delayed(false)
}

fn fixture_with_delayed(delayed: bool) -> (Fixture, Probe) {
    let probe = Probe::default();
    let copy = probe.clone();
    let fixture = Fixture::with_root([360, 160], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(copy.clone(), delayed).into_widget()),
            false,
            true,
            false,
        );
        run_app(
            app,
            WidgetsApp::new(inset_embedder::Color::new(0xff000000))
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| Overlay::new().initial_entries([entry]).into_widget())
                .into_widget(),
        );
    });
    (fixture, probe)
}

fn move_pointer(f: &mut Fixture, from: Offset, to: Offset) {
    let mut app = f.cell.borrow_mut();
    GestureBinding::instance(&mut app).handle_pointer_data_packet(
        &mut app,
        PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Move,
            kind: PointerDeviceKind::Mouse,
            time_stamp: f.at,
            pointer_identifier: f.mouse_pointer,
            physical_x: to.dx(),
            physical_y: to.dy(),
            physical_delta_x: to.dx() - from.dx(),
            physical_delta_y: to.dy() - from.dy(),
            buttons: 1,
            ..Default::default()
        }]),
    );
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

#[test]
fn drag_accepts_matching_data_and_cleans_up_feedback() {
    let (mut f, probe) = fixture();
    let start = Offset::new(50.0, 50.0);
    let end = Offset::new(240.0, 50.0);
    f.send_mouse(PointerChange::Down, start, 1);
    move_pointer(&mut f, start, end);
    f.find("Candidate");
    f.find("Feedback");
    f.send_mouse(PointerChange::Up, end, 0);
    f.pump();
    f.find("Target");
    assert_eq!(*probe.events.borrow(), ["start", "accept 42", "complete"]);
}

#[test]
fn active_drag_survives_source_unmount_and_can_cancel() {
    let (mut f, probe) = fixture();
    let start = Offset::new(50.0, 50.0);
    let middle = Offset::new(120.0, 50.0);
    f.send_mouse(PointerChange::Down, start, 1);
    move_pointer(&mut f, start, middle);
    probe
        .state
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |state| state.show_source = false);
    f.pump();
    f.find("Feedback");
    f.send_mouse(PointerChange::Up, middle, 0);
    f.pump();
    assert_eq!(*probe.events.borrow(), ["start", "cancel"]);
}

#[test]
fn delayed_drag_waits_and_movement_before_timeout_rejects_it() {
    let (mut f, probe) = fixture_with_delayed(true);
    let start = Offset::new(50.0, 50.0);
    let end = Offset::new(240.0, 50.0);
    f.send_mouse(PointerChange::Down, start, 1);
    f.cell.elapse(std::time::Duration::from_millis(499));
    f.pump();
    assert!(probe.events.borrow().is_empty());
    f.cell.elapse(std::time::Duration::from_millis(1));
    f.pump();
    assert_eq!(*probe.events.borrow(), ["start"]);
    move_pointer(&mut f, start, end);
    f.send_mouse(PointerChange::Up, end, 0);
    f.pump();
    assert_eq!(*probe.events.borrow(), ["start", "accept 42", "complete"]);

    probe.events.borrow_mut().clear();
    f.send_mouse(PointerChange::Down, start, 1);
    move_pointer(&mut f, start, end);
    f.cell.elapse(std::time::Duration::from_secs(1));
    f.send_mouse(PointerChange::Up, end, 0);
    f.pump();
    assert!(probe.events.borrow().is_empty());
}

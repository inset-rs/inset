//! SelectorBar's single selection by click and keys, its tab stop, and the pill under the
//! selected view.
#![feature(arbitrary_self_types)]
mod common;

use common::Fixture;
use inset_embedder::{Offset, PointerChange, TextDirection};
use inset_foundation::{App, Handle};
use inset_rendering::CrossAxisAlignment;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical,
};
use inset_widgets::*;
use inset_winui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

const VIEWS: [&str; 4] = ["Recent", "Shared", "Favorites", "Archived"];
const DISABLED: usize = 1;

#[derive(Debug, Default)]
struct Probe {
    state: Cell<Option<Handle<OwnerState>>>,
    asked: RefCell<Vec<Option<usize>>>,
}

/// A bar whose selection the owner keeps, as an application does.
#[derive(Debug)]
struct Owner(Rc<Probe>, Option<usize>, TextDirection);

struct OwnerState {
    state: StateData<Owner>,
    selected: Option<usize>,
}

impl StatefulWidget for Owner {
    type State = OwnerState;

    fn create_state(&self) -> Self::State {
        OwnerState {
            state: StateData::new(),
            selected: self.1,
        }
    }
}

impl State for OwnerState {
    type Widget = Owner;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.widget(app).0.state.set(Some(self));
    }

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let probe = self.widget(app).0.clone();
        let items = VIEWS
            .iter()
            .enumerate()
            .map(|(index, name)| SelectorBarItem::new(*name, *name).is_enabled(index != DISABLED))
            .collect();
        let bar = SelectorBar::new(items, app.get(self).selected, move |app, picked| {
            probe.asked.borrow_mut().push(picked);
            self.set_state(app, |state| state.selected = picked);
        });
        Directionality::new(
            self.widget(app).2,
            ThemeScope::new(
                Theme::Light,
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .children([bar.into_widget()]),
            ),
        )
        .into_widget()
    }
}

fn fixture(selected: Option<usize>, direction: TextDirection) -> (Fixture, Rc<Probe>) {
    let probe = Rc::new(Probe::default());
    let shared = probe.clone();
    let f = Fixture::with_root([600, 200], move |app| {
        winui_gallery::install_fonts(app);
        let content = shared.clone();
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Owner(content.clone(), selected, direction).into_widget()),
            false,
            true,
            false,
        );
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    FocusScope::new(Overlay::new().initial_entries([entry]))
                        .autofocus(true)
                        .into_widget()
                })
                .into_widget(),
        );
    });
    (f, probe)
}

fn press(f: &mut Fixture, physical: Physical, logical: Key) {
    let event = KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at));
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

fn release(f: &mut Fixture, physical: Physical, logical: Key) {
    let event = KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at));
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

fn key(f: &mut Fixture, physical: Physical, logical: Key) {
    press(f, physical, logical);
    release(f, physical, logical);
}

fn tap(f: &mut Fixture, text: &str) {
    let point = f.find(text);
    f.send(PointerChange::Down, point);
    f.at += Duration::from_millis(20);
    f.send(PointerChange::Up, point);
    f.pump();
}

/// Whether the column of pixels under `text` holds the pill: the accent's blue within 24 px below
/// the text's centre.
fn pill_under(f: &Fixture, text: &str) -> bool {
    let Offset { .. } = f.find(text);
    let center = f.find(text);
    let pixels = f.view.pixels.borrow();
    let width = f.view.size[0] as usize;
    (6..24).any(|dy| {
        let (x, y) = (
            center.dx().round() as usize,
            (center.dy() + dy as f64) as usize,
        );
        let index = (y * width + x) * 4;
        let [r, g, b] = [pixels[index], pixels[index + 1], pixels[index + 2]];
        b > 150 && r < 60 && g < 140
    })
}

fn settle(f: &mut Fixture) {
    for _ in 0..12 {
        f.frame_after(Duration::from_millis(20));
    }
}

#[test]
fn a_click_selects_the_view_and_grows_its_pill_in() {
    let (mut f, probe) = fixture(Some(0), TextDirection::Ltr);
    settle(&mut f);
    assert!(pill_under(&f, "Recent"));
    tap(&mut f, "Favorites");
    settle(&mut f);
    assert_eq!(*probe.asked.borrow(), [Some(2)]);
    assert!(pill_under(&f, "Favorites"));
    assert!(
        !pill_under(&f, "Recent"),
        "the pill leaves the view at once"
    );
    f.capture("selector-bar");
}

#[test]
fn a_disabled_view_takes_no_click() {
    let (mut f, probe) = fixture(Some(0), TextDirection::Ltr);
    tap(&mut f, "Shared");
    assert!(probe.asked.borrow().is_empty());
}

#[test]
fn control_click_deselects_the_selected_view() {
    let (mut f, probe) = fixture(Some(0), TextDirection::Ltr);
    press(&mut f, Physical::CONTROL_LEFT, Key::CONTROL_LEFT);
    tap(&mut f, "Recent");
    release(&mut f, Physical::CONTROL_LEFT, Key::CONTROL_LEFT);
    assert_eq!(probe.asked.borrow().last(), Some(&None));
}

#[test]
fn focus_reaching_a_bar_with_nothing_selected_selects_its_first_view() {
    let (mut f, probe) = fixture(None, TextDirection::Ltr);
    key(&mut f, Physical::TAB, Key::TAB);
    assert_eq!(*probe.asked.borrow(), [Some(0)]);
}

#[test]
fn the_arrow_keys_move_the_selection_past_a_disabled_view() {
    let (mut f, probe) = fixture(Some(0), TextDirection::Ltr);
    key(&mut f, Physical::TAB, Key::TAB);
    assert!(
        probe.asked.borrow().is_empty(),
        "Tab stops at the selected view"
    );
    key(&mut f, Physical::ARROW_RIGHT, Key::ARROW_RIGHT);
    key(&mut f, Physical::END, Key::END);
    key(&mut f, Physical::ARROW_RIGHT, Key::ARROW_RIGHT);
    key(&mut f, Physical::HOME, Key::HOME);
    assert_eq!(
        *probe.asked.borrow(),
        [Some(2), Some(3), Some(0)],
        "Right skips the disabled view, the last stays the last"
    );
}

#[test]
fn left_and_right_follow_the_reading_direction() {
    let (mut f, probe) = fixture(Some(2), TextDirection::Rtl);
    key(&mut f, Physical::TAB, Key::TAB);
    key(&mut f, Physical::ARROW_LEFT, Key::ARROW_LEFT);
    assert_eq!(*probe.asked.borrow(), [Some(3)]);
}

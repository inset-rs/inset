//! Shared SplitButton input, ToggleSplitButton checked states and flyout independence.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_embedder::{Offset, PointerChange};
use inset_foundation::{App, Handle, Listener};
use inset_painting::Alignment;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Default)]
struct Probe {
    state: Cell<Option<Handle<OwnerState>>>,
    events: RefCell<Vec<&'static str>>,
}

#[derive(Debug)]
struct Owner {
    probe: Rc<Probe>,
    toggle: bool,
}

impl std::fmt::Debug for Probe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Probe").finish_non_exhaustive()
    }
}

struct OwnerState {
    state: StateData<Owner>,
    checked: bool,
    enabled: bool,
    theme: Theme,
    flyout: Option<Handle<Flyout>>,
    focus: Option<AnyFocusNode>,
    next_focus: Option<AnyFocusNode>,
}

impl StatefulWidget for Owner {
    type State = OwnerState;

    fn create_state(&self) -> Self::State {
        OwnerState {
            state: StateData::new(),
            checked: false,
            enabled: true,
            theme: Theme::Light,
            flyout: None,
            focus: None,
            next_focus: None,
        }
    }
}

impl State for OwnerState {
    type Widget = Owner;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.widget(app).probe.state.set(Some(self));
        app.get_mut(self).flyout = Some(Flyout::new(app, Text::new("Menu choice")));
        app.get_mut(self).focus = Some(FocusNode::new(app).as_node());
        app.get_mut(self).next_focus = Some(FocusNode::new(app).as_node());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).flyout.unwrap().dispose(app);
        for node in [
            app.get(self).focus.unwrap(),
            app.get(self).next_focus.unwrap(),
        ] {
            node.dispose(app);
            app.destroy(node.id());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let probe = self.widget(app).probe.clone();
        let click = Listener::new(move |_| probe.events.borrow_mut().push("click"));
        let flyout = app.get(self).flyout.unwrap();
        let focus = app.get(self).focus.unwrap();
        let enabled = app.get(self).enabled;
        let button = if self.widget(app).toggle {
            ToggleSplitButton::text("Primary", app.get(self).checked, move |app, checked| {
                self.set_state(app, |state| state.checked = checked);
                self.widget(app).probe.events.borrow_mut().push("checked");
            })
            .click(click)
            .flyout(flyout)
            .focus_node(focus)
            .is_enabled(enabled)
            .into_widget()
        } else {
            SplitButton::text("Primary", click)
                .flyout(flyout)
                .focus_node(focus)
                .is_enabled(enabled)
                .into_widget()
        };
        ThemeScope::new(
            app.get(self).theme,
            Align::new().alignment(Alignment::TOP_LEFT.into()).child(
                Column::new()
                    .cross_axis_alignment(inset_rendering::CrossAxisAlignment::Start)
                    .children([
                        SizedBox::new().width(180.0).child(button).into_widget(),
                        Button::text("Next", Listener::new(|_| {}))
                            .focus_node(app.get(self).next_focus.unwrap())
                            .into_widget(),
                    ]),
            ),
        )
        .into_widget()
    }
}

fn fixture(toggle: bool) -> (Fixture, Rc<Probe>) {
    let probe = Rc::new(Probe::default());
    let shared = probe.clone();
    let f = Fixture::with_root([400, 300], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                Owner {
                    probe: shared.clone(),
                    toggle,
                }
                .into_widget()
            }),
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

fn key(f: &mut Fixture, physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) {
    for event in [
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
        KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)),
    ] {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
        drop(app);
        f.cell.checkpoint();
    }
    f.pump();
}

#[test]
fn primary_toggles_before_click_and_secondary_only_opens() {
    let (mut f, probe) = fixture(true);
    let state = probe.state.get().unwrap();
    f.tap("Primary");
    assert!(f.cell.borrow().get(state).checked);
    assert_eq!(&*probe.events.borrow(), &["checked", "click"]);
    f.capture("toggle_split_button_checked_light");
    let point = Offset::new(162.0, f.find("Primary").dy());
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    f.pump();
    f.find("Menu choice");
    assert_eq!(probe.events.borrow().len(), 2);
    assert!(f.cell.borrow().get(state).checked);
    key(
        &mut f,
        PhysicalKeyboardKey::ESCAPE,
        LogicalKeyboardKey::ESCAPE,
    );
    state.set_state(&mut f.cell.borrow_mut(), |s| s.theme = Theme::Dark);
    f.pump();
    f.capture("toggle_split_button_checked_dark");
    f.tap("Primary");
    assert!(!f.cell.borrow().get(state).checked);
}

#[test]
fn both_classes_have_one_tab_stop_and_keyboard_flyout_activation() {
    for toggle in [false, true] {
        let (mut f, probe) = fixture(toggle);
        let state = probe.state.get().unwrap();
        key(&mut f, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
        let focus = f.cell.borrow().get(state).focus.unwrap();
        assert!(focus.has_primary_focus(&f.cell.borrow()));
        key(
            &mut f,
            PhysicalKeyboardKey::SPACE,
            LogicalKeyboardKey::SPACE,
        );
        assert_eq!(probe.events.borrow().last(), Some(&"click"));
        assert_eq!(f.cell.borrow().get(state).checked, toggle);
        let event_count = probe.events.borrow().len();
        key(&mut f, PhysicalKeyboardKey::F4, LogicalKeyboardKey::F4);
        f.find("Menu choice");
        assert_eq!(probe.events.borrow().len(), event_count);
        key(
            &mut f,
            PhysicalKeyboardKey::ESCAPE,
            LogicalKeyboardKey::ESCAPE,
        );
        assert!(focus.has_primary_focus(&f.cell.borrow()));
        key(&mut f, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
        assert!(
            f.cell
                .borrow()
                .get(state)
                .next_focus
                .unwrap()
                .has_primary_focus(&f.cell.borrow())
        );
    }
}

#[test]
fn disabled_checked_button_ignores_both_pointer_actions() {
    let (mut f, probe) = fixture(true);
    let state = probe.state.get().unwrap();
    state.set_state(&mut f.cell.borrow_mut(), |s| {
        s.checked = true;
        s.enabled = false;
    });
    f.pump();
    f.tap("Primary");
    let point = Offset::new(162.0, f.find("Primary").dy());
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    f.pump();
    assert!(probe.events.borrow().is_empty());
    let app = f.cell.borrow();
    assert!(app.get(state).checked);
    assert!(!app.get(state).flyout.unwrap().is_open(&app));
}

#[test]
fn gallery_pages_expose_primary_and_menu_actions() {
    for (feature, label) in [
        (winui_gallery::Feature::SplitButton, "Save"),
        (winui_gallery::Feature::ToggleSplitButton, "Bold"),
        (winui_gallery::Feature::MenuFlyout, "Commands"),
    ] {
        let mut f = Fixture::for_feature([1100, 900], feature);
        f.tap(label);
        if feature == winui_gallery::Feature::MenuFlyout {
            f.tap("Autosave");
            f.find("Autosave: true · layout: List");
            f.tap("Export");
            f.find("PDF document");
            f.capture("menu_flyout_gallery");
            f.tap("PDF document");
            f.find("Last command: PDF document");
        } else {
            f.find(if feature == winui_gallery::Feature::SplitButton {
                "Primary actions: 1 · checked: false"
            } else {
                "Primary actions: 1 · checked: true"
            });
            f.capture(feature.title());
        }
    }
}

#[test]
fn checked_hover_and_press_change_only_the_targeted_half() {
    let (mut f, probe) = fixture(true);
    let state = probe.state.get().unwrap();
    state.set_state(&mut f.cell.borrow_mut(), |s| s.checked = true);
    f.pump();
    let samples = |f: &Fixture| {
        let pixels = f.view.pixels.borrow();
        [5usize, 151].map(|x| {
            let index = (14 * 400 + x) * 4;
            pixels[index..index + 4].to_vec()
        })
    };
    let normal = samples(&f);
    let primary = f.find("Primary");
    f.send_mouse(PointerChange::Add, Offset::new(250.0, 100.0), 0);
    f.send_mouse(PointerChange::Hover, primary, 0);
    f.pump();
    let hovered = samples(&f);
    assert_ne!(hovered[0], normal[0]);
    assert_eq!(hovered[1], normal[1]);
    f.send_mouse(PointerChange::Down, primary, 1);
    f.pump();
    let pressed = samples(&f);
    assert_ne!(pressed[0], hovered[0]);
    assert_eq!(pressed[1], normal[1]);
    f.capture("toggle_split_button_primary_pressed");
    f.send_mouse(PointerChange::Cancel, primary, 0);
    f.send_mouse(PointerChange::Hover, Offset::new(162.0, primary.dy()), 0);
    f.pump();
    let secondary = samples(&f);
    assert_eq!(secondary[0], normal[0]);
    assert_ne!(secondary[1], normal[1]);
    assert!(probe.events.borrow().is_empty());
}

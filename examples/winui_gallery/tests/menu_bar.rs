//! MenuBar pointer grouping, focus order, collection lifetime and source menu positioning.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_embedder::{Offset, PointerChange, TextDirection};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::{CrossAxisAlignment, RenderParagraph};
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

#[derive(Default, Debug)]
struct Probe {
    state: Cell<Option<Handle<OwnerState>>>,
    actions: Cell<usize>,
    outside: Cell<usize>,
    raw_outside: Cell<usize>,
}

#[derive(Debug)]
struct Owner {
    probe: Rc<Probe>,
    direction: TextDirection,
}

struct OwnerState {
    state: StateData<Owner>,
    show_file: bool,
    renamed: bool,
    reversed: bool,
    dark: bool,
    enabled: bool,
    file_enabled: bool,
    top: f64,
}

impl StatefulWidget for Owner {
    type State = OwnerState;
    fn create_state(&self) -> Self::State {
        OwnerState {
            state: StateData::new(),
            show_file: true,
            renamed: false,
            reversed: false,
            dark: false,
            enabled: true,
            file_enabled: true,
            top: 0.0,
        }
    }
}

impl State for OwnerState {
    type Widget = Owner;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.widget(app).probe.state.set(Some(self));
    }

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let probe = self.widget(app).probe.clone();
        let action = |text: &'static str| {
            let probe = probe.clone();
            MenuFlyoutItem::new(
                text,
                Listener::new(move |_| probe.actions.set(probe.actions.get() + 1)),
            )
        };
        let mut items = vec![
            MenuBarItem::new("disabled", "Unavailable", vec![action("Disabled action")])
                .is_enabled(false),
            MenuBarItem::new("edit", "Edit", vec![action("Copy item")]),
            MenuBarItem::new("view", "View", vec![action("Zoom item")]),
        ];
        if app.get(self).show_file {
            items.insert(
                0,
                MenuBarItem::new(
                    "file",
                    if app.get(self).renamed {
                        "Files"
                    } else {
                        "File"
                    },
                    vec![
                        action("New document"),
                        MenuFlyoutItem::sub_item("Export", vec![action("Deep command")]),
                    ],
                )
                .is_enabled(app.get(self).file_enabled),
            );
        }
        if app.get(self).reversed {
            items.reverse();
        }
        let outside = probe.clone();
        let raw_outside = probe.clone();
        Directionality::new(
            self.widget(app).direction,
            ThemeScope::new(
                if app.get(self).dark {
                    Theme::Dark
                } else {
                    Theme::Light
                },
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children([
                        SizedBox::new().height(app.get(self).top).into_widget(),
                        MenuBar::new(items)
                            .is_enabled(app.get(self).enabled)
                            .into_widget(),
                        SizedBox::new().height(80.0).into_widget(),
                        inset_widgets::Listener::new()
                            .on_pointer_down(Rc::new(move |_, _| {
                                raw_outside
                                    .raw_outside
                                    .set(raw_outside.raw_outside.get() + 1)
                            }))
                            .child(Button::text(
                                "Outside",
                                Listener::new(move |_| {
                                    outside.outside.set(outside.outside.get() + 1)
                                }),
                            ))
                            .into_widget(),
                    ]),
            ),
        )
        .into_widget()
    }
}

fn fixture(direction: TextDirection) -> (Fixture, Rc<Probe>) {
    let probe = Rc::new(Probe::default());
    let shared = probe.clone();
    let f = Fixture::with_root([600, 400], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                Owner {
                    probe: shared.clone(),
                    direction,
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

fn focused(f: &Fixture, text: &str) -> bool {
    let point = f.find(text);
    let mut app = f.cell.borrow_mut();
    let Some(node) = primary_focus(&mut app) else {
        return false;
    };
    let Some(context) = node.context(&app) else {
        return false;
    };
    let Some(render) = context.find_render_object(&app).and_then(|r| r.as_box()) else {
        return false;
    };
    let bounds = render.local_to_global(&app, Offset::ZERO, None) & render.size(&app);
    bounds.height() <= 40.0 && bounds.contains(point)
}

fn contains_text(f: &Fixture, text: &str) -> bool {
    f.elements().into_iter().any(|element| {
        let app = f.cell.borrow();
        element
            .render_object(&app)
            .and_then(|render| render.downcast::<RenderParagraph>(&app))
            .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == text)
    })
}

#[test]
fn pointer_switches_headers_and_outside_taps_are_consumed() {
    let (mut f, probe) = fixture(TextDirection::Ltr);
    f.send_mouse(PointerChange::Add, Offset::new(500.0, 100.0), 0);
    let file = f.find("File");
    f.send_mouse(PointerChange::Hover, file, 0);
    f.send_mouse(PointerChange::Down, file, 1);
    f.send_mouse(PointerChange::Up, file, 0);
    f.pump();
    f.find("New document");
    let edit = f.find("Edit");
    f.send_mouse(PointerChange::Hover, edit, 0);
    f.pump();
    f.find("Copy item");
    f.capture("menu_bar_hover_switch");
    f.tap("Outside");
    assert_eq!(probe.outside.get(), 0);
    assert_eq!(
        probe.raw_outside.get(),
        1,
        "TapRegion consumes gestures, not raw pointer observation"
    );
    assert!(focused(&f, "Edit"));
    f.tap("Outside");
    assert_eq!(probe.outside.get(), 1);
    f.tap("File");
    f.tap("New document");
    assert_eq!(probe.actions.get(), 1);
}

#[test]
fn tab_enters_once_and_open_menu_arrows_skip_disabled_headers() {
    let (mut f, _) = fixture(TextDirection::Ltr);
    key(&mut f, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
    assert!(focused(&f, "File"));
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    assert!(
        focused(&f, "File"),
        "closed-header MoveFocusTo tries only the immediate neighbor"
    );
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_LEFT,
        LogicalKeyboardKey::ARROW_LEFT,
    );
    assert!(focused(&f, "View"));
    key(&mut f, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
    assert!(focused(&f, "Outside"));
    f.tap("File");
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    f.find("Copy item");
    key(
        &mut f,
        PhysicalKeyboardKey::ESCAPE,
        LogicalKeyboardKey::ESCAPE,
    );
    assert!(focused(&f, "Edit"));
}

#[test]
fn an_open_menu_can_switch_after_its_header_is_disabled() {
    let (mut f, probe) = fixture(TextDirection::Ltr);
    f.tap("File");
    probe
        .state
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| {
            s.file_enabled = false;
        });
    f.pump();
    f.find("New document");

    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    f.find("Copy item");
}

#[test]
fn rtl_reverses_header_navigation_and_aligns_the_popup_to_the_right() {
    let (mut f, _) = fixture(TextDirection::Rtl);
    key(&mut f, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
    assert!(focused(&f, "File"));
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    assert!(focused(&f, "View"));
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_DOWN,
        LogicalKeyboardKey::ARROW_DOWN,
    );
    f.find("Zoom item");
    f.capture("menu_bar_rtl");
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_LEFT,
        LogicalKeyboardKey::ARROW_LEFT,
    );
    f.find("New document");
}

#[test]
fn switching_menus_cancels_an_old_submenu_open_timer() {
    let (mut f, _) = fixture(TextDirection::Ltr);
    f.tap("File");
    f.send_mouse(PointerChange::Add, Offset::new(500.0, 100.0), 0);
    let export = f.find("Export");
    f.send_mouse(PointerChange::Hover, export, 0);
    f.pump();
    let edit = f.find("Edit");
    f.send_mouse(PointerChange::Hover, edit, 0);
    f.pump();
    f.find("Copy item");
    f.cell.elapse(Duration::from_millis(500));
    f.pump();
    assert!(
        !contains_text(&f, "Deep command"),
        "a closed menu cannot later open its old submenu"
    );
}

#[test]
fn item_updates_reordering_and_removal_preserve_valid_menu_lifetimes() {
    let (mut f, probe) = fixture(TextDirection::Ltr);
    f.tap("File");
    let state = probe.state.get().unwrap();
    state.set_state(&mut f.cell.borrow_mut(), |s| {
        s.renamed = true;
        s.reversed = true;
        s.dark = true;
    });
    f.pump();
    f.find("Files");
    f.find("New document");
    f.capture("menu_bar_updated_dark");
    state.set_state(&mut f.cell.borrow_mut(), |s| s.show_file = false);
    f.pump();
    f.cell.elapse(Duration::from_millis(500));
    f.pump();
    f.tap("Edit");
    f.tap("Copy item");
    assert_eq!(probe.actions.get(), 1);
}

#[test]
fn first_touch_opens_above_the_header_when_there_is_room() {
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let (mut f, probe) = fixture(direction);
        let state = probe.state.get().unwrap();
        state.set_state(&mut f.cell.borrow_mut(), |s| s.top = 180.0);
        f.pump();
        let header = f.find("File");
        f.tap("File");
        assert!(
            f.find("New document").dy() < header.dy(),
            "touch menus exclude their header in {direction:?}"
        );
        f.capture(if direction == TextDirection::Ltr {
            "menu_bar_touch_above"
        } else {
            "menu_bar_touch_above_rtl"
        });
    }
}

#[test]
fn gallery_updates_checked_items_direction_and_header_collection() {
    let mut f = Fixture::for_feature([1100, 900], winui_gallery::Feature::MenuBar);
    f.tap("File");
    f.tap("Autosave");
    f.find("Autosave: true");
    f.tap("Export");
    f.find("PDF document");
    f.capture("menu_bar_gallery_light");
    f.tap("PDF document");
    f.find("Last command: PDF document");
    f.tap("Direction: LTR");
    f.find("Direction: RTL");
    f.tap("Hide View");
    f.tap("Dark theme");
    f.tap("File");
    f.find("New document");
    f.capture("menu_bar_gallery_dark_rtl");
}

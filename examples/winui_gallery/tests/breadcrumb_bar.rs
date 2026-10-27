//! Source overflow order, native retained layout, input, and collection ownership.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_embedder::{Offset, PointerChange, TextDirection};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::{CrossAxisAlignment, RenderParagraph};
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical,
};
use inset_widgets::*;
use inset_winui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug, Default)]
struct Probe {
    state: Cell<Option<Handle<OwnerState>>>,
    clicked: RefCell<Vec<(usize, String)>>,
    mounted: Cell<usize>,
    disposed: Cell<usize>,
}

#[derive(Debug)]
struct Owner(Rc<Probe>, TextDirection);

struct OwnerState {
    state: StateData<Owner>,
    width: f64,
    count: usize,
    dark: bool,
    first_enabled: bool,
}

impl StatefulWidget for Owner {
    type State = OwnerState;

    fn create_state(&self) -> Self::State {
        OwnerState {
            state: StateData::new(),
            width: 500.0,
            count: 4,
            dark: false,
            first_enabled: true,
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
        let items = ["Root", "Documents", "Projects", "Current"]
            .iter()
            .take(app.get(self).count)
            .enumerate()
            .map(|(i, label)| {
                if i == 0 {
                    BreadcrumbBarItem::new(i.to_string(), RetainedLabel(probe.clone()))
                        .is_enabled(app.get(self).first_enabled)
                } else {
                    BreadcrumbBarItem::text(i.to_string(), *label)
                }
            })
            .collect();
        Directionality::new(
            self.widget(app).1,
            ThemeScope::new(
                if app.get(self).dark {
                    Theme::Dark
                } else {
                    Theme::Light
                },
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .children([
                        SizedBox::new()
                            .width(app.get(self).width)
                            .child(BreadcrumbBar::new(items, move |_, args| {
                                probe.clicked.borrow_mut().push((args.index, args.item.id))
                            }))
                            .into_widget(),
                        SizedBox::new().height(200.0).into_widget(),
                        Button::text("Outside", Listener::new(|_| {})).into_widget(),
                    ]),
            ),
        )
        .into_widget()
    }
}

#[derive(Debug)]
struct RetainedLabel(Rc<Probe>);

struct RetainedLabelState {
    state: StateData<RetainedLabel>,
}

impl StatefulWidget for RetainedLabel {
    type State = RetainedLabelState;

    fn create_state(&self) -> Self::State {
        RetainedLabelState {
            state: StateData::new(),
        }
    }
}

impl State for RetainedLabelState {
    type Widget = RetainedLabel;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let probe = &self.widget(app).0;
        probe.mounted.set(probe.mounted.get() + 1);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let probe = &self.widget(app).0;
        probe.disposed.set(probe.disposed.get() + 1);
    }

    fn build(self: Handle<Self>, _: &mut App, _: BuildContext) -> WidgetRef {
        Text::new("Root").into_widget()
    }
}

fn fixture(direction: TextDirection) -> (Fixture, Rc<Probe>) {
    let probe = Rc::new(Probe::default());
    let shared = probe.clone();
    let f = Fixture::with_root([600, 400], move |app| {
        winui_gallery::install_fonts(app);
        let content = shared.clone();
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Owner(content.clone(), direction).into_widget()),
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

fn key(f: &mut Fixture, physical: Physical, logical: Key) {
    for event in [
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
        KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)),
    ] {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
        drop(app);
        f.cell.checkpoint();
        f.pump();
    }
}

fn tap_point(f: &mut Fixture, point: Offset) {
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    f.pump();
}

fn ellipsis(f: &Fixture) -> Offset {
    let elements = f.elements();
    let app = f.cell.borrow();
    let element = elements
        .into_iter()
        .find(|element| {
            downcast_widget::<FontIcon>(element.widget(&app).as_ref())
                .is_some_and(|icon| icon.glyph == FluentSymbol::More.glyph().to_string())
        })
        .unwrap();
    let render = element.find_render_object(&app).unwrap().as_box().unwrap();
    render.local_to_global(
        &app,
        Offset::new(
            render.size(&app).width() / 2.0,
            render.size(&app).height() / 2.0,
        ),
        None,
    )
}

/// The overflow copy appears below its retained inline counterpart.
fn popup_label(f: &Fixture, text: &str) -> Offset {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements
        .into_iter()
        .filter_map(|element| {
            let render = element.render_object(&app)?;
            let paragraph = render.downcast::<RenderParagraph>(&app)?;
            if paragraph.text(&app).to_plain_text(true, true) != text {
                return None;
            }
            let render = render.as_box()?;
            Some(render.local_to_global(
                &app,
                Offset::new(
                    render.size(&app).width() / 2.0,
                    render.size(&app).height() / 2.0,
                ),
                None,
            ))
        })
        .max_by(|a, b| a.dy().total_cmp(&b.dy()))
        .unwrap()
}

#[test]
fn overflow_reverses_ancestors_and_reports_original_indices_in_both_directions() {
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let (mut f, probe) = fixture(direction);
        probe
            .state
            .get()
            .unwrap()
            .set_state(&mut f.cell.borrow_mut(), |s| s.width = 100.0);
        f.pump();
        let point = ellipsis(&f);
        tap_point(&mut f, point);
        let projects = popup_label(&f, "Projects");
        let documents = popup_label(&f, "Documents");
        let root = popup_label(&f, "Root");
        assert!(projects.dy() < documents.dy() && documents.dy() < root.dy());
        f.capture(if direction == TextDirection::Ltr {
            "breadcrumb_overflow_ltr"
        } else {
            "breadcrumb_overflow_rtl"
        });
        tap_point(&mut f, documents);
        assert_eq!(&*probe.clicked.borrow(), &[(1, "1".into())]);
        f.find("Current");
    }
}

#[test]
fn inline_pointer_and_keyboard_follow_current_item_rules() {
    let (mut f, probe) = fixture(TextDirection::Ltr);
    f.tap("Root");
    f.tap("Current");
    assert_eq!(
        probe.clicked.borrow().len(),
        1,
        "current item has no pointer button"
    );
    key(&mut f, Physical::TAB, Key::TAB);
    for _ in 0..3 {
        key(&mut f, Physical::ARROW_RIGHT, Key::ARROW_RIGHT);
    }
    key(&mut f, Physical::ENTER, Key::ENTER);
    assert_eq!(probe.clicked.borrow().last().unwrap().0, 3);
    key(&mut f, Physical::TAB, Key::TAB);
    f.capture("breadcrumb_keyboard_exit");
}

#[test]
fn resizing_collection_changes_and_theme_keep_the_bar_usable() {
    let (mut f, probe) = fixture(TextDirection::Ltr);
    let state = probe.state.get().unwrap();
    for width in [100.0, 500.0, 60.0, 300.0, 100.0] {
        state.set_state(&mut f.cell.borrow_mut(), |s| s.width = width);
        f.pump();
    }
    let point = ellipsis(&f);
    tap_point(&mut f, point);
    key(&mut f, Physical::ESCAPE, Key::ESCAPE);
    state.set_state(&mut f.cell.borrow_mut(), |s| {
        s.dark = true;
        s.width = 500.0;
        s.count = 2;
    });
    f.pump();
    f.tap("Root");
    assert_eq!(probe.clicked.borrow().last().unwrap().0, 0);
    f.capture("breadcrumb_short_dark");
    state.set_state(&mut f.cell.borrow_mut(), |s| s.count = 0);
    f.pump();
    f.tap("Outside");
}

#[test]
fn gallery_exposes_navigation_overflow_and_custom_content() {
    let mut f = Fixture::for_feature([1100, 900], winui_gallery::Feature::BreadcrumbBar);
    f.capture("breadcrumb_gallery_light");
    f.tap("Projects");
    f.find("Invoked: Projects (index 2)");
    f.tap("Reset path");
    f.tap("Use narrow layout");
    f.tap("Direction: LTR");
    f.tap("Dark theme");
    f.capture("breadcrumb_gallery_dark_rtl");
}

#[test]
fn hidden_custom_content_retains_state_and_disabled_first_item_does_not_block_tab() {
    let (mut f, probe) = fixture(TextDirection::Ltr);
    let state = probe.state.get().unwrap();
    assert_eq!(probe.mounted.get(), 1);
    for width in [80.0, 500.0, 100.0, 500.0] {
        state.set_state(&mut f.cell.borrow_mut(), |s| s.width = width);
        f.pump();
        assert_eq!(probe.mounted.get(), 1);
        assert_eq!(probe.disposed.get(), 0);
    }
    state.set_state(&mut f.cell.borrow_mut(), |s| s.first_enabled = false);
    f.pump();
    key(&mut f, Physical::TAB, Key::TAB);
    key(&mut f, Physical::ENTER, Key::ENTER);
    assert_eq!(probe.clicked.borrow().last().unwrap().0, 1);
}

#[test]
fn keyboard_enters_overflow_and_moves_to_the_next_ancestor() {
    let (mut f, probe) = fixture(TextDirection::Ltr);
    probe
        .state
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.width = 100.0);
    f.pump();
    key(&mut f, Physical::TAB, Key::TAB);
    key(&mut f, Physical::ENTER, Key::ENTER);
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    key(&mut f, Physical::ENTER, Key::ENTER);
    assert_eq!(probe.clicked.borrow().last().unwrap().0, 1);
}

//! The system focus visual follows XAML's coerced focus state: programmatic focus after pointer input shows no ring, after keyboard input it does.
#![feature(arbitrary_self_types)]

use inset_embedder::{Offset, PointerChange, TextDirection};
use inset_foundation::{Handle, Listener};
use inset_rendering::CrossAxisAlignment;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::rc::Rc;

/// A menu bar whose header takes focus when its menu opens, above a plain button.
fn fixture() -> Fixture {
    Fixture::with_root([600, 400], |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(|_, _| {
                Directionality::new(
                    TextDirection::Ltr,
                    ThemeScope::new(
                        Theme::Light,
                        Column::new()
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .children([
                                MenuBar::new(vec![
                                    MenuBarItem::new(
                                        "file",
                                        "File",
                                        vec![MenuFlyoutItem::new(
                                            "New document",
                                            Listener::new(|_| {}),
                                        )],
                                    ),
                                    MenuBarItem::new(
                                        "edit",
                                        "Edit",
                                        vec![MenuFlyoutItem::new(
                                            "Copy item",
                                            Listener::new(|_| {}),
                                        )],
                                    ),
                                ])
                                .into_widget(),
                                SizedBox::new().height(80.0).into_widget(),
                                Button::text("Outside", Listener::new(|_| {})).into_widget(),
                            ]),
                    ),
                )
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
    })
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

fn click(f: &mut Fixture, text: &str) {
    let point = f.find(text);
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
}

/// Whether the focused node's box contains `point`.
fn focus_contains(f: &Fixture, point: Offset) -> bool {
    let mut app = f.cell.borrow_mut();
    let Some(node) = primary_focus(&mut app) else {
        return false;
    };
    let Some(render) = node
        .context(&app)
        .and_then(|context| context.find_render_object(&app))
        .and_then(|render| render.as_box())
    else {
        return false;
    };
    (render.local_to_global(&app, Offset::ZERO, None) & render.size(&app)).contains(point)
}

/// How many focus visuals are currently drawn.
fn visible_rings(f: &Fixture) -> usize {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements
        .into_iter()
        .filter(|element| {
            downcast_widget::<FocusVisual>(element.widget(&app).as_ref())
                .is_some_and(|visual| visual.visible)
        })
        .count()
}

#[test]
fn mouse_opened_menu_focuses_without_focus_visuals() {
    let mut f = fixture();
    f.send_mouse(PointerChange::Add, Offset::new(500.0, 300.0), 0);
    click(&mut f, "File");
    let file = f.find("File");
    let item = f.find("New document");
    assert!(
        focus_contains(&f, file) || focus_contains(&f, item),
        "opening the flyout moves focus into the header or its menu"
    );
    assert_eq!(
        visible_rings(&f),
        0,
        "pointer-caused programmatic focus draws no ring"
    );
    f.send_mouse(PointerChange::Hover, item, 0);
    f.pump();
    assert_eq!(visible_rings(&f), 0);
}

#[test]
fn keyboard_opened_menu_shows_focus_visuals_on_header_and_first_item() {
    let mut f = fixture();
    key(&mut f, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
    let file = f.find("File");
    assert!(focus_contains(&f, file));
    assert_eq!(visible_rings(&f), 1, "keyboard focus draws the header ring");
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_DOWN,
        LogicalKeyboardKey::ARROW_DOWN,
    );
    let item = f.find("New document");
    assert!(
        focus_contains(&f, item),
        "a keyboard-opened menu focuses its first item"
    );
    assert_eq!(visible_rings(&f), 1, "the ring moves to the focused item");
}

#[test]
fn keyboard_focus_ring_survives_later_pointer_input() {
    let mut f = fixture();
    key(&mut f, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
    assert_eq!(visible_rings(&f), 1);
    f.send_mouse(PointerChange::Add, Offset::new(500.0, 300.0), 0);
    f.send_mouse(PointerChange::Hover, Offset::new(500.0, 350.0), 0);
    f.send_mouse(PointerChange::Down, Offset::new(500.0, 350.0), 1);
    f.send_mouse(PointerChange::Up, Offset::new(500.0, 350.0), 0);
    f.pump();
    let file = f.find("File");
    assert!(
        focus_contains(&f, file),
        "a click on empty space leaves focus where it was"
    );
    assert_eq!(
        visible_rings(&f),
        1,
        "the focus state is latched when focus arrives, not re-read later"
    );
}

#[test]
fn arrow_key_on_the_only_item_shows_its_ring() {
    let mut f = fixture();
    f.send_mouse(PointerChange::Add, Offset::new(500.0, 300.0), 0);
    click(&mut f, "File");
    let item = f.find("New document");
    assert!(focus_contains(&f, item));
    assert_eq!(visible_rings(&f), 0);
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_DOWN,
        LogicalKeyboardKey::ARROW_DOWN,
    );
    assert!(
        focus_contains(&f, item),
        "a one-item menu cycles back to the same item"
    );
    assert_eq!(
        visible_rings(&f),
        1,
        "CycleFocus re-focuses with FocusState_Keyboard even on the same item"
    );
}

/// A button whose focus node the test holds, above a SelectorBar of two views.
fn controls_fixture() -> (Fixture, Handle<FocusNode>) {
    let slot = Rc::new(std::cell::Cell::new(None));
    let node = slot.clone();
    let f = Fixture::with_root([600, 300], move |app| {
        winui_gallery::install_fonts(app);
        let focus = FocusNode::new(app);
        node.set(Some(focus));
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                let views = vec![
                    SelectorBarItem::new("recent", "Recent"),
                    SelectorBarItem::new("shared", "Shared"),
                ];
                Directionality::new(
                    TextDirection::Ltr,
                    ThemeScope::new(
                        Theme::Light,
                        Column::new()
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .children([
                                Button::text("Target", Listener::new(|_| {}))
                                    .focus_node(focus.as_node())
                                    .into_widget(),
                                SizedBox::new().height(60.0).into_widget(),
                                Text::new("Elsewhere").into_widget(),
                                SelectorBar::new(views, Some(0), |_, _| {}).into_widget(),
                            ]),
                    ),
                )
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
    let node = slot.get().expect("made by the fixture");
    (f, node)
}

/// Focuses `node` from code, as a dialog focuses its button when it opens.
fn focus_by_code(f: &mut Fixture, node: Handle<FocusNode>) {
    node.request_focus(&mut f.cell.borrow_mut(), None);
    f.cell.checkpoint();
    f.pump();
}

#[test]
fn a_button_focused_by_code_after_a_click_draws_no_ring() {
    let (mut f, node) = controls_fixture();
    f.send_mouse(PointerChange::Add, Offset::new(500.0, 250.0), 0);
    click(&mut f, "Elsewhere");
    focus_by_code(&mut f, node);
    assert!(focus_contains(&f, f.find("Target")));
    assert_eq!(
        visible_rings(&f),
        0,
        "programmatic focus after a click is pointer focus"
    );
}

#[test]
fn a_button_focused_by_code_after_a_key_draws_its_ring() {
    let (mut f, node) = controls_fixture();
    key(
        &mut f,
        PhysicalKeyboardKey::SHIFT_LEFT,
        LogicalKeyboardKey::SHIFT_LEFT,
    );
    focus_by_code(&mut f, node);
    assert!(focus_contains(&f, f.find("Target")));
    assert_eq!(
        visible_rings(&f),
        1,
        "programmatic focus after a key is keyboard focus"
    );
}

#[test]
fn a_selector_bar_view_clicked_draws_no_ring_and_arrowed_to_does() {
    let (mut f, _) = controls_fixture();
    f.send_mouse(PointerChange::Add, Offset::new(500.0, 250.0), 0);
    click(&mut f, "Recent");
    assert!(
        focus_contains(&f, f.find("Recent")),
        "a pressed view takes the focus"
    );
    assert_eq!(visible_rings(&f), 0);
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    assert!(focus_contains(&f, f.find("Shared")));
    assert_eq!(visible_rings(&f), 1);
}

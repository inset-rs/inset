//! MenuFlyout templates, checked values and source keyboard navigation.
#![feature(arbitrary_self_types)]

use inset_foundation::{Handle, Listener};
use inset_painting::Alignment;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc};

#[derive(Default)]
struct Probe {
    menu: Cell<Option<Handle<MenuFlyout>>>,
    calls: Cell<usize>,
    checked: Cell<bool>,
    radio: Cell<usize>,
}

fn items(probe: Rc<Probe>) -> Vec<MenuFlyoutItem> {
    let invoke = probe.clone();
    let toggle = probe.clone();
    let first = probe.clone();
    let second = probe.clone();
    let child = probe.clone();
    vec![
        MenuFlyoutItem::new(
            "Open document",
            Listener::new(move |_| invoke.calls.set(invoke.calls.get() + 1)),
        )
        .icon(FontIcon::symbol(FluentSymbol::Add))
        .keyboard_accelerator_text_override("Ctrl+O"),
        MenuFlyoutItem::separator(),
        MenuFlyoutItem::new(
            "Unavailable",
            Listener::new(|_| panic!("disabled item invoked")),
        )
        .is_enabled(false),
        MenuFlyoutItem::toggle("Autosave", probe.checked.get(), move |app, checked| {
            toggle.checked.set(checked);
            toggle.menu.get().unwrap().items(app, items(toggle.clone()));
        })
        .prevent_dismiss_on_pointer(true),
        MenuFlyoutItem::radio(
            "List view",
            probe.radio.get() == 0,
            Listener::new(move |app| {
                first.radio.set(0);
                first.menu.get().unwrap().items(app, items(first.clone()));
            }),
        )
        .prevent_dismiss_on_pointer(true),
        MenuFlyoutItem::radio(
            "Grid view",
            probe.radio.get() == 1,
            Listener::new(move |app| {
                second.radio.set(1);
                second.menu.get().unwrap().items(app, items(second.clone()));
            }),
        )
        .prevent_dismiss_on_pointer(true),
        MenuFlyoutItem::sub_item(
            "More commands",
            vec![
                MenuFlyoutItem::new(
                    "Child action",
                    Listener::new(move |_| child.calls.set(child.calls.get() + 1)),
                ),
                MenuFlyoutItem::sub_item(
                    "One more level",
                    vec![MenuFlyoutItem::new("Deep action", Listener::new(|_| {}))],
                ),
            ],
        ),
    ]
}

fn fixture() -> (Fixture, Rc<Probe>) {
    let probe = Rc::new(Probe::default());
    let shared = probe.clone();
    let fixture = Fixture::with_root([600, 600], move |app| {
        winui_gallery::install_fonts(app);
        let menu = MenuFlyout::new(app, items(shared.clone()));
        shared.menu.set(Some(menu));
        let flyout = menu
            .as_flyout(app)
            .placement(app, FlyoutPlacementMode::BottomEdgeAlignedLeft);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                ThemeScope::new(
                    Theme::Light,
                    Align::new()
                        .alignment(Alignment::TOP_LEFT.into())
                        .child(DropDownButton::text("Commands").flyout(flyout)),
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
    (fixture, probe)
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
    for element in f.elements() {
        let mut app = f.cell.borrow_mut();
        if let Some(object) = element.render_object(&app)
            && let Some(paragraph) = object.downcast::<inset_rendering::RenderParagraph>(&app)
            && paragraph.text(&app).to_plain_text(true, true) == text
        {
            return Focus::of(&mut app, element, false, false).has_primary_focus(&app);
        }
    }
    false
}

#[test]
fn command_invocation_closes_the_menu() {
    let (mut f, probe) = fixture();
    f.tap("Commands");
    f.capture("menu_flyout_touch");
    f.tap("Open document");
    assert_eq!(probe.calls.get(), 1);
    assert!(!probe.menu.get().unwrap().is_open(&f.cell.borrow()));
}

#[test]
fn checked_items_update_without_closing_when_requested() {
    let (mut f, probe) = fixture();
    f.tap("Commands");
    f.tap("Autosave");
    assert!(probe.checked.get());
    f.tap("Grid view");
    assert_eq!(probe.radio.get(), 1);
    assert!(probe.menu.get().unwrap().is_open(&f.cell.borrow()));
    f.capture("menu_flyout_checked");
}

#[test]
fn keyboard_skips_disabled_items_and_returns_from_submenu() {
    let (mut f, _) = fixture();
    f.focus("Commands");
    key(
        &mut f,
        PhysicalKeyboardKey::SPACE,
        LogicalKeyboardKey::SPACE,
    );
    assert!(focused(&f, "Open document"));
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_DOWN,
        LogicalKeyboardKey::ARROW_DOWN,
    );
    assert!(focused(&f, "Autosave"));
    for _ in 0..3 {
        key(
            &mut f,
            PhysicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ARROW_DOWN,
        );
    }
    assert!(focused(&f, "More commands"));
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    f.find("Child action");
    assert!(focused(&f, "Child action"));
    f.capture("menu_flyout_cascade");
    key(
        &mut f,
        PhysicalKeyboardKey::ARROW_LEFT,
        LogicalKeyboardKey::ARROW_LEFT,
    );
    assert!(focused(&f, "More commands"));
}

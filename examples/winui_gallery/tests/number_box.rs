//! NumberBox: expressions committed by Enter and focus loss, validation and coercion, the keys,
//! the wheel and both kinds of spin buttons, and how the box treats the value the application
//! owns.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_embedder::{
    Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind,
    Size, TextAffinity, TextInputAction, TextRange, TextSelection,
};
use inset_foundation::{App, Handle, ValueNotifier};
use inset_gestures::GestureBinding;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyRepeatEvent, KeyUpEvent,
    LogicalKeyboardKey as Key, PhysicalKeyboardKey as Physical, TextEditingValue, TextInput,
};
use inset_widgets::*;
use inset_winui::*;
use std::{cell::RefCell, rc::Rc};
use winui_gallery::Feature;

/// A gallery page tall enough to show every example without scrolling.
const PAGE: [u32; 2] = [1080, 1700];

/// The text box of the NumberBox with `placeholder`: its text, its focus and a point inside it.
struct InputBox {
    controller: AnyTextEditingController,
    focus: Handle<FocusNode>,
    point: Offset,
}

/// The element of the NumberBox whose placeholder is `placeholder`.
fn number_box_element(f: &Fixture, placeholder: &str) -> AnyElement {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements
        .into_iter()
        .find(|element| {
            downcast_widget::<NumberBox>(element.widget(&app).as_ref())
                .is_some_and(|number_box| number_box.placeholder_text == placeholder)
        })
        .unwrap_or_else(|| panic!("missing NumberBox {placeholder}"))
}

/// The elements beneath `root`, root first.
fn descendants(f: &Fixture, root: AnyElement) -> Vec<AnyElement> {
    let app = f.cell.borrow();
    let mut all = vec![root];
    let mut index = 0;
    while index < all.len() {
        all.extend(all[index].children(&app));
        index += 1;
    }
    all
}

fn input_box(f: &Fixture, placeholder: &str) -> InputBox {
    let root = number_box_element(f, placeholder);
    let elements = descendants(f, root);
    let app = f.cell.borrow();
    for element in elements {
        if let Some(editable) = downcast_widget::<EditableText>(element.widget(&app).as_ref()) {
            let render = element.find_render_object(&app).unwrap().as_box().unwrap();
            return InputBox {
                controller: editable.controller,
                focus: editable.focus_node,
                point: render.local_to_global(&app, Offset::new(8.0, 8.0), None),
            };
        }
    }
    panic!("missing InputBox of {placeholder}")
}

/// A repeat button on screen: its center, size and whether it can be pressed.
#[derive(Clone, Copy, Debug)]
struct SpinButton {
    center: Offset,
    size: Size,
    enabled: bool,
}

/// The repeat buttons among `elements`, top to bottom and then left to right.
fn spin_buttons(f: &Fixture, elements: Vec<AnyElement>) -> Vec<SpinButton> {
    let app = f.cell.borrow();
    let mut buttons: Vec<_> = elements
        .into_iter()
        .filter_map(|element| {
            let widget = element.widget(&app);
            let button = downcast_widget::<RepeatButton>(widget.as_ref())?;
            let render = element.find_render_object(&app)?.as_box()?;
            let size = render.size(&app);
            Some(SpinButton {
                center: render.local_to_global(
                    &app,
                    Offset::new(size.width() / 2.0, size.height() / 2.0),
                    None,
                ),
                size,
                enabled: button.is_enabled,
            })
        })
        .collect();
    buttons.sort_by(|a, b| {
        (a.center.dy(), a.center.dx())
            .partial_cmp(&(b.center.dy(), b.center.dx()))
            .unwrap()
    });
    buttons
}

/// The inline up and down buttons of the NumberBox with `placeholder`.
fn inline_spin_buttons(f: &Fixture, placeholder: &str) -> Vec<SpinButton> {
    let root = number_box_element(f, placeholder);
    let elements = descendants(f, root);
    spin_buttons(f, elements)
}

/// The popup's up and down buttons, which live in the root overlay.
fn popup_spin_buttons(f: &Fixture) -> Vec<SpinButton> {
    let elements = f.elements();
    spin_buttons(f, elements)
        .into_iter()
        .filter(|button| button.size == Size::new(36.0, 36.0))
        .collect()
}

fn click(f: &mut Fixture, point: Offset) {
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
}

/// The editing value a host's input method would send: `text` with the caret at its end.
fn type_text(f: &mut Fixture, text: &str) {
    let value = TextEditingValue::new()
        .text(text)
        .selection(TextSelection::collapsed(
            text.encode_utf16().count() as i32,
            TextAffinity::Downstream,
        ))
        .composing(TextRange::EMPTY);
    let mut app = f.cell.borrow_mut();
    TextInput::instance(&mut app).update_editing_value(&mut app, value);
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

fn send_key(f: &mut Fixture, event: KeyEvent) {
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
}

/// A press and release of `logical`.
fn key(f: &mut Fixture, logical: Key, physical: Physical) {
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
    );
    send_key(f, KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)));
    f.pump();
}

/// A mouse wheel turned over `point`; a negative `dy` scrolls up.
fn wheel(f: &mut Fixture, point: Offset, dy: f64) {
    let mut app = f.cell.borrow_mut();
    GestureBinding::instance(&mut app).handle_pointer_data_packet(
        &mut app,
        PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Hover,
            kind: PointerDeviceKind::Mouse,
            signal_kind: Some(PointerSignalKind::Scroll),
            physical_x: point.dx(),
            physical_y: point.dy(),
            scroll_delta_y: dy,
            ..Default::default()
        }]),
    );
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

fn text(f: &Fixture, input: &InputBox) -> String {
    input.controller.text_value(&f.cell.borrow()).to_owned()
}

fn focused(f: &Fixture, input: &InputBox) -> bool {
    input.focus.has_focus(&mut f.cell.borrow_mut())
}

#[test]
fn an_expression_is_computed_when_enter_commits_it_and_the_focus_stays() {
    let mut f = Fixture::for_feature(PAGE, Feature::NumberBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let input = input_box(&f, "1 + 2^2");
    click(&mut f, input.point);
    assert!(focused(&f, &input));
    type_text(&mut f, "1 + 2^2");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(text(&f, &input), "5");
    f.find("Expression value: 5");
    f.find("ValueChanged: NaN → 5");
    assert!(focused(&f, &input), "Enter keeps the focus");
    // The platform's done action for Enter leaves the focus where it is too.
    {
        let mut app = f.cell.borrow_mut();
        TextInput::instance(&mut app).perform_action(&mut app, TextInputAction::Done);
    }
    f.cell.checkpoint();
    f.pump();
    assert!(focused(&f, &input));
    // An expression that does not parse is replaced by the value.
    type_text(&mut f, "2 * (3");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(text(&f, &input), "5");
    // Precedence, and a division by zero, which clears the value.
    type_text(&mut f, "(2 + 3) * 4 - 2^3");
    key(&mut f, Key::NUMPAD_ENTER, Physical::NUMPAD_ENTER);
    f.find("Expression value: 12");
    type_text(&mut f, "1/0");
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Expression value: NaN");
    assert_eq!(text(&f, &input), "");
}

#[test]
fn leaving_the_box_commits_it_and_invalid_input_is_overwritten_unless_validation_is_disabled() {
    let mut f = Fixture::for_feature(PAGE, Feature::NumberBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let percentage = input_box(&f, "0 to 100");
    let expression = input_box(&f, "1 + 2^2");
    assert_eq!(text(&f, &percentage), "50");
    click(&mut f, percentage.point);
    assert_eq!(
        percentage.controller.selection(&f.cell.borrow()),
        TextSelection::new(0, 2),
        "taking the focus selects the text"
    );
    type_text(&mut f, "abc");
    click(&mut f, expression.point);
    assert!(!focused(&f, &percentage));
    assert_eq!(text(&f, &percentage), "50");
    // A value out of range goes to the bound it passed.
    click(&mut f, percentage.point);
    type_text(&mut f, "150");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(text(&f, &percentage), "100");
    f.find("Percentage value: 100");
    f.find("ValueChanged: 50 → 100");
    // Clearing the text clears the value.
    type_text(&mut f, "  ");
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Percentage value: NaN");
    // With validation disabled, invalid text and values out of range stay.
    f.tap("Overwrite invalid input");
    click(&mut f, percentage.point);
    type_text(&mut f, "250");
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Percentage value: 250");
    type_text(&mut f, "abc");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(text(&f, &percentage), "abc");
    f.find("Percentage value: 250");
}

#[test]
fn the_keys_and_the_wheel_step_while_focused_and_escape_restores_the_text() {
    let mut f = Fixture::for_feature(PAGE, Feature::NumberBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let input = input_box(&f, "0 to 500");
    // Unfocused, the wheel leaves the value alone.
    wheel(&mut f, input.point, -100.0);
    assert_eq!(text(&f, &input), "0");
    click(&mut f, input.point);
    key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    assert_eq!(text(&f, &input), "10");
    assert_eq!(
        input.controller.selection(&f.cell.borrow()),
        TextSelection::collapsed(2, TextAffinity::Downstream),
        "a step puts the caret at the end"
    );
    key(&mut f, Key::PAGE_UP, Physical::PAGE_UP);
    assert_eq!(text(&f, &input), "110");
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    assert_eq!(text(&f, &input), "100");
    // Each repeat of a held key steps again, even before the application rebuilds.
    let at = f.at;
    send_key(
        &mut f,
        KeyEvent::Down(KeyDownEvent::new(Physical::ARROW_UP, Key::ARROW_UP, at)),
    );
    send_key(
        &mut f,
        KeyEvent::Repeat(KeyRepeatEvent::new(Physical::ARROW_UP, Key::ARROW_UP, at)),
    );
    send_key(
        &mut f,
        KeyEvent::Up(KeyUpEvent::new(Physical::ARROW_UP, Key::ARROW_UP, at)),
    );
    f.pump();
    assert_eq!(text(&f, &input), "120");
    key(&mut f, Key::PAGE_DOWN, Physical::PAGE_DOWN);
    assert_eq!(text(&f, &input), "20");
    f.find("Integer value: 20");
    // Typed text is committed before a step.
    type_text(&mut f, "300");
    key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    assert_eq!(text(&f, &input), "310");
    wheel(&mut f, input.point, -100.0);
    assert_eq!(text(&f, &input), "320");
    wheel(&mut f, input.point, 100.0);
    assert_eq!(text(&f, &input), "310");
    f.find("Integer value: 310");
    // Escape puts the value back in place of what was typed.
    type_text(&mut f, "77");
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
    assert_eq!(text(&f, &input), "310");
    f.find("Integer value: 310");
}

#[test]
fn inline_spin_buttons_step_and_stop_at_the_bounds_unless_wrapping() {
    let mut f = Fixture::for_feature(PAGE, Feature::NumberBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let input = input_box(&f, "0 to 500");
    let buttons = inline_spin_buttons(&f, "0 to 500");
    assert_eq!(buttons.len(), 2);
    let (up, down) = (buttons[0], buttons[1]);
    assert_eq!(up.size.width(), 32.0);
    assert!(
        up.enabled && !down.enabled,
        "at the minimum only up can step"
    );
    click(&mut f, up.center);
    assert_eq!(text(&f, &input), "10");
    assert!(!focused(&f, &input), "spin buttons do not take the focus");
    assert!(inline_spin_buttons(&f, "0 to 500")[1].enabled);
    click(&mut f, input.point);
    type_text(&mut f, "500");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert!(!inline_spin_buttons(&f, "0 to 500")[0].enabled);
    f.capture("number_box_light");
    f.tap("Wrap");
    let up = inline_spin_buttons(&f, "0 to 500")[0];
    assert!(up.enabled, "wrapping enables both directions");
    click(&mut f, input.point);
    click(&mut f, up.center);
    assert_eq!(text(&f, &input), "0");
    assert!(
        focused(&f, &input),
        "the box keeps the focus a spin button got"
    );
    f.find("ValueChanged: 500 → 0");
    let disabled = inline_spin_buttons(&f, "Disabled");
    assert!(disabled.iter().all(|button| !button.enabled));
    f.tap("Dark theme");
    f.capture("number_box_dark");
}

#[test]
fn compact_spin_buttons_open_in_a_popup_while_the_box_has_the_focus() {
    let mut f = Fixture::for_feature(PAGE, Feature::NumberBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    f.tap("Spin buttons: Inline");
    f.find("Spin buttons: Compact");
    assert!(inline_spin_buttons(&f, "0 to 500").is_empty());
    let input = input_box(&f, "0 to 500");
    assert!(popup_spin_buttons(&f).is_empty());
    click(&mut f, input.point);
    let buttons = popup_spin_buttons(&f);
    assert_eq!(buttons.len(), 2, "focus opens the popup");
    assert!(buttons[0].enabled && !buttons[1].enabled);
    assert_eq!(buttons[1].center.dy() - buttons[0].center.dy(), 40.0);
    f.capture("number_box_popup_light");
    click(&mut f, buttons[0].center);
    assert_eq!(text(&f, &input), "10");
    assert!(focused(&f, &input), "the popup keeps the box focused");
    // The popup's buttons repeat while held.
    f.send_mouse(PointerChange::Down, buttons[0].center, 1);
    for _ in 0..40 {
        f.cell.elapse(std::time::Duration::from_millis(20));
    }
    f.send_mouse(PointerChange::Up, buttons[0].center, 0);
    f.pump();
    let repeated: f64 = text(&f, &input).parse().unwrap();
    assert!(
        repeated > 20.0,
        "held for 800 ms, the button repeats: {repeated}"
    );
    f.tap("Dark theme");
    let input = input_box(&f, "0 to 500");
    click(&mut f, input.point);
    f.capture("number_box_popup_dark");
    let amount = input_box(&f, "0.00");
    click(&mut f, amount.point);
    assert!(!focused(&f, &input));
    assert_eq!(
        popup_spin_buttons(&f).len(),
        2,
        "only the focused box has a popup"
    );
    assert_eq!(
        text(&f, &amount),
        "1.50",
        "the custom formatter shows two decimals"
    );
    let up = popup_spin_buttons(&f)[0];
    click(&mut f, up.center);
    assert_eq!(text(&f, &amount), "1.75");
}

/// A window with one NumberBox whose value lives in `value`, built by `make`; whether the
/// application takes what the box asks for is `accept`.
struct Host {
    value: Handle<ValueNotifier<f64>>,
    events: Rc<RefCell<Vec<(f64, f64)>>>,
}

fn host(
    initial: f64,
    accept: bool,
    make: impl Fn(NumberBox) -> NumberBox + 'static,
) -> (Fixture, Rc<RefCell<Option<Host>>>) {
    let shared = Rc::new(RefCell::new(None));
    let slot = shared.clone();
    let fixture = Fixture::with_root([480, 320], move |app: &mut App| {
        winui_gallery::install_fonts(app);
        let value = app.create(ValueNotifier::new(initial));
        let events = Rc::new(RefCell::new(Vec::new()));
        *slot.borrow_mut() = Some(Host {
            value,
            events: events.clone(),
        });
        let make = Rc::new(make);
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .page_route_builder(|app, settings, builder| {
                    let route = PageRouteBuilder::new(
                        app,
                        Rc::new(move |app, context, _, _| builder(app, context)),
                    )
                    .settings(app, RouteSettingsRef::Settings(settings.clone()));
                    PageRoute::as_page_route(route)
                })
                .home(ThemeScope::new(
                    Theme::Light,
                    ValueListenableBuilder::new(Rc::new(value), move |_, _, current, _| {
                        let events = events.clone();
                        let number_box = NumberBox::new(*current, move |app, args| {
                            events.borrow_mut().push((args.old_value, args.new_value));
                            if accept {
                                value.set_value(app, args.new_value);
                            }
                        })
                        .placeholder_text("host");
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_START)
                            .child(SizedBox::new().width(200.0).child(make(number_box)))
                            .into_widget()
                    }),
                ))
                .into_widget(),
        );
    });
    (fixture, shared)
}

fn events(host: &Rc<RefCell<Option<Host>>>) -> Vec<(f64, f64)> {
    host.borrow().as_ref().unwrap().events.borrow().clone()
}

#[test]
fn a_value_out_of_range_is_coerced_and_reported_after_the_frame() {
    let (mut f, host) = host(150.0, true, |number_box| number_box.maximum(100.0));
    let input = input_box(&f, "host");
    assert_eq!(text(&f, &input), "100");
    let reported = events(&host);
    assert_eq!(reported.len(), 1);
    assert!(reported[0].0.is_nan() && reported[0].1 == 100.0);
    let value = host.borrow().as_ref().unwrap().value;
    assert_eq!(*f.cell.borrow().get(value).value(), 100.0);
    // A value the application sets within range raises no ValueChanged.
    value.set_value(&mut f.cell.borrow_mut(), 40.0);
    f.cell.checkpoint();
    f.pump();
    assert_eq!(text(&f, &input), "40");
    assert_eq!(events(&host).len(), 1);
}

#[test]
fn a_value_the_application_does_not_take_is_undone() {
    let (mut f, host) = host(5.0, false, |number_box| number_box);
    let input = input_box(&f, "host");
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    click(&mut f, input.point);
    key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    assert_eq!(events(&host), vec![(5.0, 6.0)]);
    assert_eq!(text(&f, &input), "5", "the application kept 5");
    type_text(&mut f, "9");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(events(&host).last(), Some(&(5.0, 9.0)));
    assert_eq!(text(&f, &input), "5");
}

#[test]
fn text_from_the_application_is_read_as_committed_input() {
    let (mut f, host) = host(f64::NAN, true, |number_box| {
        number_box.accepts_expression(true).text("2 + 3")
    });
    f.pump();
    let input = input_box(&f, "host");
    assert_eq!(text(&f, &input), "5");
    let reported = events(&host);
    assert_eq!(reported.len(), 1);
    assert!(reported[0].0.is_nan() && reported[0].1 == 5.0);
}

#[test]
fn an_equal_formatter_built_again_leaves_typed_text_alone() {
    let (mut f, host) = host(5.0, true, |number_box| {
        number_box.number_formatter(DecimalFormatter {
            integer_digits: 1,
            fraction_digits: 2,
        })
    });
    let input = input_box(&f, "host");
    assert_eq!(text(&f, &input), "5.00");
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    click(&mut f, input.point);
    type_text(&mut f, "12");
    // The parent builds the box again, with a new but equal formatter, while "12" is typed.
    let value = host.borrow().as_ref().unwrap().value;
    value.notify_listeners(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    f.pump();
    assert_eq!(text(&f, &input), "12", "the typed text stays");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(text(&f, &input), "12.00");
}

/// Like WinUI's template (`PopupIndicator` is `VerticalAlignment` Center), a box with compact spin
/// buttons given more height than it needs keeps the height it needs.
#[test]
fn a_compact_box_given_spare_height_keeps_its_own() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let size = common::size_under_center([300, 300], |_| {
        NumberBox::new(1.0, |_, _| {})
            .spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Compact)
            .into_widget()
    });
    assert_eq!(size.height(), r.text_control_theme_min_height);
}

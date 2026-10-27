//! ComboBox: opening and closing by pointer and keyboard, where the drop-down opens, selecting
//! and moving through items, type-ahead search, light dismiss, the long lazy list, the editable
//! box's matching and TextSubmitted, and how the box treats the selection the application owns.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::{Fixture, mount};
use inset_embedder::{
    Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind,
    Rect, TextAffinity, TextRange, TextSelection,
};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::GestureBinding;
use inset_rendering::RenderParagraph;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical, TextEditingValue, TextInput,
};
use inset_widgets::*;
use inset_winui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use winui_gallery::Feature;

/// A gallery page tall enough to show every example without scrolling.
const PAGE: [u32; 2] = [1080, 1500];

/// The element of the ComboBox whose placeholder is `placeholder`.
fn combo_box_element(f: &Fixture, placeholder: &str) -> AnyElement {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements
        .into_iter()
        .find(|element| {
            downcast_widget::<ComboBox>(element.widget(&app).as_ref())
                .is_some_and(|combo_box| combo_box.placeholder_text == placeholder)
        })
        .unwrap_or_else(|| panic!("missing ComboBox {placeholder}"))
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

/// The bounds of the render box of `element`.
fn bounds(f: &Fixture, element: AnyElement) -> Rect {
    let app = f.cell.borrow();
    let render = element.find_render_object(&app).unwrap().as_box().unwrap();
    let origin = render.local_to_global(&app, Offset::ZERO, None);
    origin & render.size(&app)
}

/// The box of the ComboBox with `placeholder`: its `Background`, the first child of the drop-down's
/// portal.
fn box_bounds(f: &Fixture, placeholder: &str) -> Rect {
    let root = combo_box_element(f, placeholder);
    let portal = descendants(f, root)
        .into_iter()
        .find(|element| {
            let app = f.cell.borrow();
            downcast_widget::<OverlayPortal>(element.widget(&app).as_ref()).is_some()
        })
        .expect("the drop-down's portal");
    bounds(f, portal)
}

/// Whether `text` is on screen.
fn shows(f: &Fixture, text: &str) -> bool {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().any(|element| {
        element.render_object(&app).is_some_and(|object| {
            object
                .downcast::<RenderParagraph>(&app)
                .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == text)
                && object.as_box().unwrap().size(&app).width() > 0.0
        })
    })
}

/// The center of the left edge of the label `text`, built anywhere, also beyond what a list shows.
fn label_center(f: &Fixture, text: &str) -> Option<Offset> {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().find_map(|element| {
        let object = element.render_object(&app)?;
        let paragraph = object.downcast::<RenderParagraph>(&app)?;
        (paragraph.text(&app).to_plain_text(true, true) == text).then(|| {
            let render = object.as_box().unwrap();
            let size = render.size(&app);
            render.local_to_global(&app, Offset::new(0.0, size.height() / 2.0), None)
        })
    })
}

/// The editable box's text box: its text, its focus and a point inside it.
struct EditableText {
    controller: AnyTextEditingController,
    focus: Handle<FocusNode>,
}

fn editable_text(f: &Fixture, placeholder: &str) -> EditableText {
    let root = combo_box_element(f, placeholder);
    let elements = descendants(f, root);
    let app = f.cell.borrow();
    for element in elements {
        if let Some(editable) =
            downcast_widget::<inset_widgets::EditableText>(element.widget(&app).as_ref())
        {
            return EditableText {
                controller: editable.controller,
                focus: editable.focus_node,
            };
        }
    }
    panic!("missing EditableText of {placeholder}")
}

fn click(f: &mut Fixture, point: Offset) {
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
}

fn hover(f: &mut Fixture, point: Offset) {
    f.send_mouse(PointerChange::Hover, point, 0);
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

/// `logical` pressed while Alt is held.
fn alt_key(f: &mut Fixture, logical: Key, physical: Physical) {
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(Physical::ALT_LEFT, Key::ALT_LEFT, f.at)),
    );
    key(f, logical, physical);
    send_key(
        f,
        KeyEvent::Up(KeyUpEvent::new(Physical::ALT_LEFT, Key::ALT_LEFT, f.at)),
    );
    f.pump();
}

/// A key typing `character`, which the host reports as the key's character.
fn type_character(f: &mut Fixture, character: &str) {
    let first = character.chars().next().unwrap();
    let logical = Key::new(first.to_ascii_lowercase() as u64);
    let physical = Physical::KEY_A;
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at).character(character)),
    );
    send_key(f, KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)));
    f.pump();
}

/// A key typing `character` in a text box, followed by the edit the host's input method makes:
/// `text` with the caret at its end.
fn type_in_text_box(f: &mut Fixture, character: &str, text: &str) {
    let first = character.chars().next().unwrap();
    let logical = Key::new(first.to_ascii_lowercase() as u64);
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(Physical::DIGIT1, logical, f.at).character(character)),
    );
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
    send_key(
        f,
        KeyEvent::Up(KeyUpEvent::new(Physical::DIGIT1, logical, f.at)),
    );
    f.pump();
}

/// The color at `point` of the last frame.
fn pixel(f: &Fixture, point: Offset) -> [u8; 4] {
    let pixels = f.view.pixels.borrow();
    let index = (point.dy() as usize * f.view.size[0] as usize + point.dx() as usize) * 4;
    [
        pixels[index],
        pixels[index + 1],
        pixels[index + 2],
        pixels[index + 3],
    ]
}

/// A mouse wheel turned over `point`; a negative `dy` turns it away from the user.
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

fn gallery() -> Fixture {
    let mut f = Fixture::for_feature(PAGE, Feature::ComboBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    f
}

#[test]
fn the_page_closed_and_open_in_both_themes() {
    let mut f = gallery();
    f.capture("combo_box_closed_light");
    let colors = box_bounds(&f, "Pick a color");
    click(&mut f, colors.center());
    f.capture("combo_box_open_light");
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
    let fonts = box_bounds(&f, "Pick a font");
    click(&mut f, fonts.center());
    f.capture("combo_box_long_list_light");
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
    f.tap("Dark theme");
    f.capture("combo_box_closed_dark");
    click(&mut f, colors.center());
    f.capture("combo_box_open_dark");
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
}

#[test]
fn a_click_opens_the_box_and_a_click_on_an_item_selects_it_and_closes_it() {
    let mut f = gallery();
    let colors = box_bounds(&f, "Pick a color");
    assert!(shows(&f, "Pick a color"));
    assert!(!shows(&f, "Yellow"));
    click(&mut f, colors.center());
    assert!(shows(&f, "Yellow"), "the drop-down is open");
    // With nothing selected, the middle item is over the box.
    let red = f.find("Red");
    assert!(
        (red.dy() - colors.center().dy()).abs() < 2.0,
        "{red:?} {colors:?}"
    );
    let point = f.find("Green");
    click(&mut f, point);
    assert!(!shows(&f, "Yellow"), "the drop-down closed");
    f.find("Color: Green");
    f.find("Colors SelectionChanged: nothing → Green");
    // The box shows the selected item; reopened, the selected item is over the box.
    assert!(shows(&f, "Green"));
    click(&mut f, colors.center());
    let green = f.find("Green");
    assert!((green.dy() - colors.center().dy()).abs() < 2.0);
    // Ctrl and a click on the selected item clears the selection.
    let at = f.at;
    send_key(
        &mut f,
        KeyEvent::Down(KeyDownEvent::new(
            Physical::CONTROL_LEFT,
            Key::CONTROL_LEFT,
            at,
        )),
    );
    click(&mut f, green);
    let at = f.at;
    send_key(
        &mut f,
        KeyEvent::Up(KeyUpEvent::new(
            Physical::CONTROL_LEFT,
            Key::CONTROL_LEFT,
            at,
        )),
    );
    f.pump();
    f.find("Color: none");
    assert!(shows(&f, "Pick a color"));
}

#[test]
fn a_press_outside_the_drop_down_cancels_it_and_goes_no_further() {
    let mut f = gallery();
    let colors = box_bounds(&f, "Pick a color");
    click(&mut f, colors.center());
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    // The light-dismiss canvas covers the page: the check box beneath stays unchecked.
    let check = f.find("Handle TextSubmitted");
    click(&mut f, check);
    assert!(!shows(&f, "Yellow"), "the drop-down closed");
    f.find("Color: none");
    assert_eq!(handles_submitted(&f), Some(false));
    // A press on the page now reaches it.
    click(&mut f, check);
    assert_eq!(handles_submitted(&f), Some(true));
}

/// The value of the gallery's "Handle TextSubmitted" check box.
fn handles_submitted(f: &Fixture) -> Option<bool> {
    f.elements()
        .into_iter()
        .find_map(|element| {
            let app = f.cell.borrow();
            downcast_widget::<CheckBox>(element.widget(&app).as_ref()).map(|c| c.is_checked)
        })
        .unwrap()
}

#[test]
fn the_keyboard_opens_and_closes_the_box() {
    let mut f = gallery();
    f.focus("Pick a color");
    key(&mut f, Key::F4, Physical::F4);
    assert!(shows(&f, "Yellow"), "F4 opens");
    key(&mut f, Key::F4, Physical::F4);
    assert!(!shows(&f, "Yellow"), "F4 closes");
    alt_key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    assert!(shows(&f, "Yellow"), "Alt+Down opens");
    alt_key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    assert!(!shows(&f, "Yellow"), "Alt+Up closes");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert!(shows(&f, "Yellow"), "Enter opens");
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
    assert!(!shows(&f, "Yellow"), "Escape closes");
    key(&mut f, Key::SPACE, Physical::SPACE);
    assert!(shows(&f, "Yellow"), "Space opens");
    f.capture("combo_box_keyboard_open_light");
    // Tab closes it and moves on.
    key(&mut f, Key::TAB, Physical::TAB);
    assert!(!shows(&f, "Yellow"), "Tab closes");
    f.find("Color: none");
}

#[test]
fn the_arrows_select_while_closed_and_move_the_highlight_while_open() {
    let mut f = gallery();
    f.focus("Pick a color");
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    f.find("Color: Blue");
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    f.find("Color: Green");
    key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    f.find("Color: Blue");
    key(&mut f, Key::END, Physical::END);
    f.find("Color: Yellow");
    key(&mut f, Key::HOME, Physical::HOME);
    f.find("Color: Blue");
    // Open, the arrows move the highlight only; Enter selects it.
    key(&mut f, Key::F4, Physical::F4);
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    f.find("Color: Blue");
    f.capture("combo_box_highlight_light");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert!(!shows(&f, "Yellow"));
    f.find("Color: Red");
    // Escape restores what was selected when the box opened.
    key(&mut f, Key::F4, Physical::F4);
    key(&mut f, Key::END, Physical::END);
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
    f.find("Color: Red");
}

#[test]
fn typing_selects_the_item_whose_text_starts_with_what_was_typed() {
    let mut f = gallery();
    f.focus("Pick a color");
    type_character(&mut f, "y");
    f.find("Color: Yellow");
    // A second later, a new search starts after the selection and wraps.
    f.frame_after(Duration::from_millis(1100));
    type_character(&mut f, "g");
    f.find("Color: Green");
    f.frame_after(Duration::from_millis(1100));
    // Typed within a second, the characters make one search string.
    type_character(&mut f, "b");
    type_character(&mut f, "l");
    f.find("Color: Blue");
    // While searching, Space is typed rather than opening the box.
    type_character(&mut f, "u");
    key(&mut f, Key::SPACE, Physical::SPACE);
    assert!(!shows(&f, "Yellow"));
}

#[test]
fn the_long_list_opens_with_the_selected_font_over_the_box_and_pages() {
    let mut f = gallery();
    let fonts = box_bounds(&f, "Pick a font");
    click(&mut f, fonts.center());
    // The selected font is over the box, with seven fonts above it and seven below; those
    // beyond are outside the drop-down.
    let selected = label_center(&f, "Segoe UI").unwrap();
    assert!(
        (selected.dy() - fonts.center().dy()).abs() < 2.0,
        "{selected:?} {fonts:?}"
    );
    let step = label_center(&f, "Segoe UI Light").unwrap().dy() - selected.dy();
    let first = label_center(&f, "Segoe Print Black").unwrap();
    let last = label_center(&f, "Segoe UI Emoji Light").unwrap();
    assert!((selected.dy() - first.dy() - 7.0 * step).abs() < 1.0);
    assert!((last.dy() - selected.dy() - 7.0 * step).abs() < 1.0);
    let above = Offset::new(first.dx(), first.dy() - step);
    let below = Offset::new(last.dx(), last.dy() + step);
    f.capture("combo_box_long_list_open");
    assert_ne!(
        pixel(&f, first),
        pixel(&f, above),
        "the drop-down ends above the first"
    );
    assert_ne!(pixel(&f, last), pixel(&f, below), "and below the last");
    // Page Down moves to the last font fully shown, then a page on.
    key(&mut f, Key::PAGE_DOWN, Physical::PAGE_DOWN);
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Font: Segoe UI Emoji Light");
    click(&mut f, fonts.center());
    key(&mut f, Key::PAGE_DOWN, Physical::PAGE_DOWN);
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert!(!shows(&f, "Font: Segoe UI Emoji Light"));
    // Home goes to the first font, scrolling it into view.
    click(&mut f, fonts.center());
    key(&mut f, Key::HOME, Physical::HOME);
    assert!(shows(&f, "Arial"));
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Font: Arial");
}

#[test]
fn disabled_items_are_skipped_by_the_arrows_and_take_no_clicks() {
    let mut f = gallery();
    let shape = box_bounds(&f, "");
    click(&mut f, shape.center());
    let square = f.find("Square");
    click(&mut f, square);
    assert!(
        shows(&f, "Triangle"),
        "a click on a disabled item leaves it open"
    );
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Shape SelectionChanged: Circle → Triangle");
    // Closed, Up skips the disabled item as well.
    key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    f.find("Shape SelectionChanged: Triangle → Circle");
}

#[test]
fn the_wheel_selects_while_the_box_has_the_focus() {
    let mut f = gallery();
    let colors = box_bounds(&f, "Pick a color");
    // Unfocused, the wheel scrolls the page instead.
    wheel(&mut f, colors.center(), 100.0);
    f.find("Color: none");
    f.focus("Pick a color");
    let colors = box_bounds(&f, "Pick a color");
    wheel(&mut f, colors.center(), 100.0);
    f.find("Color: Blue");
    wheel(&mut f, colors.center(), 100.0);
    f.find("Color: Green");
    wheel(&mut f, colors.center(), -100.0);
    f.find("Color: Blue");
}

#[test]
fn the_box_and_its_items_show_the_pointer_over_and_pressed() {
    let mut f = gallery();
    let colors = box_bounds(&f, "Pick a color");
    let inside = Offset::new(colors.right - 60.0, colors.center().dy());
    let normal = pixel(&f, inside);
    hover(&mut f, inside);
    let pointer_over = pixel(&f, inside);
    assert_ne!(normal, pointer_over, "ComboBoxBackgroundPointerOver");
    f.send_mouse(PointerChange::Down, inside, 1);
    f.frame_after(Duration::from_millis(200));
    let pressed = pixel(&f, inside);
    assert_ne!(pointer_over, pressed, "ComboBoxBackgroundPressed");
    f.send_mouse(PointerChange::Up, inside, 0);
    f.pump();
    // Items: the one under the pointer, then pressed.
    let yellow = f.find("Yellow");
    let probe = Offset::new(yellow.dx() + 80.0, yellow.dy());
    let before = pixel(&f, probe);
    hover(&mut f, probe);
    let over = pixel(&f, probe);
    assert_ne!(before, over, "ComboBoxItemBackgroundPointerOver");
    f.send_mouse(PointerChange::Down, probe, 1);
    f.frame_after(Duration::from_millis(200));
    assert_ne!(over, pixel(&f, probe), "ComboBoxItemBackgroundPressed");
    f.capture("combo_box_item_pressed_light");
    f.send_mouse(PointerChange::Up, probe, 0);
    f.pump();
    f.find("Color: Yellow");
}

#[test]
fn an_editable_box_completes_typed_text_and_commits_it_or_submits_it() {
    let mut f = gallery();
    let sizes = box_bounds(&f, "Pick or type a size");
    let text = editable_text(&f, "Pick or type a size");
    click(&mut f, Offset::new(sizes.left + 20.0, sizes.center().dy()));
    assert!(
        text.focus.has_focus(&mut f.cell.borrow_mut()),
        "a click focuses the text"
    );
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "12");
    assert_eq!(
        text.controller.selection(&f.cell.borrow()),
        TextSelection::new(0, 2),
        "taking the focus selects the text"
    );
    // Typing completes to the first size that starts with it, selecting what was added.
    type_in_text_box(&mut f, "2", "2");
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "20");
    assert_eq!(
        text.controller.selection(&f.cell.borrow()),
        TextSelection::new(1, 2)
    );
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Font size: 20");
    f.find("Font size SelectionChanged: 12 → 20");
    // Text that is none of the items is submitted and kept as a custom value.
    type_in_text_box(&mut f, "1", "1");
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "10");
    {
        let mut app = f.cell.borrow_mut();
        text.controller.set_value(
            &mut app,
            TextEditingValue::new()
                .text("1")
                .selection(TextSelection::collapsed(1, TextAffinity::Downstream)),
        );
    }
    f.pump();
    type_in_text_box(&mut f, "3", "13");
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "13");
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Font size: 13 (typed)");
    f.find("Font size SelectionChanged: 20 → “13”");
    // Down opens the drop-down, below the box; the arrows go through the sizes.
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    assert!(shows(&f, "72"));
    f.capture("combo_box_editable_light");
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "8");
    key(&mut f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "9");
    // Escape reverts to the custom value.
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
    assert!(!shows(&f, "72"));
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "13");
    f.find("Font size: 13 (typed)");
}

#[test]
fn a_handled_text_submission_keeps_the_selection() {
    let mut f = gallery();
    f.tap("Handle TextSubmitted");
    let sizes = box_bounds(&f, "Pick or type a size");
    let text = editable_text(&f, "Pick or type a size");
    click(&mut f, Offset::new(sizes.left + 20.0, sizes.center().dy()));
    {
        let mut app = f.cell.borrow_mut();
        text.controller.set_value(
            &mut app,
            TextEditingValue::new()
                .text("")
                .selection(TextSelection::collapsed(0, TextAffinity::Downstream)),
        );
    }
    type_in_text_box(&mut f, "5", "5");
    key(&mut f, Key::ENTER, Physical::ENTER);
    f.find("Font size TextSubmitted: “5”, handled");
    f.find("Font size: 12");
    assert_eq!(text.controller.text_value(&f.cell.borrow()), "12");
    // Leaving the box shows its selection in place of the text box.
    let point = f.find("Colors");
    click(&mut f, point);
    assert!(!text.focus.has_focus(&mut f.cell.borrow_mut()));
    f.capture("combo_box_editable_unfocused_light");
    f.tap("Dark theme");
    click(&mut f, Offset::new(sizes.right - 15.0, sizes.center().dy()));
    assert!(shows(&f, "72"), "the arrow of an editable box opens it");
    f.capture("combo_box_editable_dark");
}

/// What the application of `Owner` asks of the box it owns.
#[derive(Default)]
struct Log(RefCell<Vec<String>>);

impl Log {
    fn push(&self, entry: impl Into<String>) {
        self.0.borrow_mut().push(entry.into());
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.0.borrow_mut())
    }
}

/// An application owning a box's selection and drop-down, which it may decline to change.
#[derive(Debug)]
struct Owner {
    log: Rc<Log>,
}

impl std::fmt::Debug for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Log")
    }
}

struct OwnerState {
    state: StateData<Owner>,
    selected: Option<usize>,
    open: bool,
    accept: bool,
}

impl StatefulWidget for Owner {
    type State = OwnerState;

    fn create_state(&self) -> OwnerState {
        OwnerState {
            state: StateData::new(),
            selected: Some(1),
            open: false,
            accept: true,
        }
    }
}

impl State for OwnerState {
    type Widget = Owner;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let log = self.widget(app).log.clone();
        let (selected, open) = (app.get(self).selected, app.get(self).open);
        let items = ["One", "Two", "Three"]
            .map(|text| ComboBoxItem::text(text, text))
            .to_vec();
        let (selection_log, open_log, opened_log, closed_log) =
            (log.clone(), log.clone(), log.clone(), log.clone());
        Column::new()
            .children([
                Button::text(
                    "Decline",
                    Listener::new(move |app| self.set_state(app, |s| s.accept = false)),
                )
                .into_widget(),
                Button::text(
                    "Open",
                    Listener::new(move |app| self.set_state(app, |s| s.open = true)),
                )
                .into_widget(),
                Button::text(
                    "Select three",
                    Listener::new(move |app| self.set_state(app, |s| s.selected = Some(2))),
                )
                .into_widget(),
                SizedBox::new()
                    .width(200.0)
                    .child(
                        ComboBox::new(items, selected, move |app, args| {
                            selection_log.push(format!("selection {:?}", args.selected_index()));
                            if app.get(self).accept {
                                self.set_state(app, |s| s.selected = args.selected_index());
                            }
                        })
                        .placeholder_text("Owned")
                        .is_drop_down_open(open, move |app, open| {
                            open_log.push(format!("open {open}"));
                            if app.get(self).accept {
                                self.set_state(app, |s| s.open = open);
                            }
                        })
                        .drop_down_opened(Listener::new(move |_| opened_log.push("opened")))
                        .drop_down_closed(Listener::new(move |_| closed_log.push("closed"))),
                    )
                    .into_widget(),
            ])
            .into_widget()
    }
}

fn owner() -> (Fixture, Rc<Log>) {
    let log = Rc::new(Log::default());
    let root = log.clone();
    let mut f = Fixture::with_root(
        [600, 600],
        mount(move |app| {
            winui_gallery::install_fonts(app);
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
                    Align::new()
                        .alignment(inset_painting::AlignmentGeometry::TOP_START)
                        .child(Owner { log: root }),
                ))
                .into_widget()
        }),
    );
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    (f, log)
}

#[test]
fn the_application_owns_the_selection_and_the_drop_down() {
    let (mut f, log) = owner();
    let owned = box_bounds(&f, "Owned");
    assert!(shows(&f, "Two"));
    click(&mut f, owned.center());
    assert_eq!(log.take(), ["open true", "opened"]);
    let point = f.find("Three");
    click(&mut f, point);
    assert_eq!(log.take(), ["selection Some(2)", "open false", "closed"]);
    assert!(shows(&f, "Three"));
    // A selection the application sets raises nothing.
    let point = f.find("Select three");
    click(&mut f, point);
    assert!(log.take().is_empty());
    // The application opening the drop-down opens it.
    let point = f.find("Open");
    click(&mut f, point);
    assert!(shows(&f, "One"));
    assert_eq!(log.take(), ["opened"]);
    key(&mut f, Key::ESCAPE, Physical::ESCAPE);
    assert_eq!(log.take(), ["open false", "closed"]);
    // A selection the application declines is undone.
    let point = f.find("Decline");
    click(&mut f, point);
    let owned = box_bounds(&f, "Owned");
    f.focus("Three");
    key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    assert_eq!(log.take(), ["selection Some(1)"]);
    assert!(
        shows(&f, "Three"),
        "the box shows the application's selection"
    );
    // And so is an opening it declines: the box closes to the application's value without
    // asking again.
    click(&mut f, owned.center());
    assert_eq!(log.take(), ["open true", "opened", "closed"]);
    assert!(!shows(&f, "One"));
}

/// Like WinUI's style (`VerticalAlignment` Top), a box given more height than it needs keeps the
/// height it needs, so a row of controls lines it up with the others.
#[test]
fn a_box_given_spare_height_keeps_its_own() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let size = common::size_under_center([300, 300], |_| {
        let items = vec![ComboBoxItem::text("One", "One")];
        ComboBox::new(items, Some(0), |_, _| {}).into_widget()
    });
    assert_eq!(size.height(), r.combo_box_min_height);
}

//! A multiline `TextBox` that does not wrap: its lines stay whole, the box scrolls them sideways,
//! and the caret stays in view on both axes, whatever moves it.

use std::{rc::Rc, time::Duration};
use waft_embedder::{
    Offset, PointerChange, PointerDeviceKind, Rect, SystemFontSource, TargetPlatform, TextAffinity,
    TextDirection, TextSelection,
};
use waft_foundation::{App, AppCell, Handle};
use waft_gestures::{
    GestureBinding, PointerDownEvent, PointerEvent, PointerMoveEvent, PointerUpEvent,
};
use waft_painting::PaintingBinding;
use waft_rendering::{RenderBox, RenderEditable, RenderHandle};
use waft_scheduler::SchedulerBinding;
use waft_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical, TextEditingValue, TextInput, TextSelectionDelegate,
};
use waft_test::{IMPLICIT_VIEW_ID, MockClipboard, TestPlatform, TestView};
use waft_widgets::*;
use waft_winui::{AccentPalette, ScrollBarVisibility, TextBox, TextBoxState, TextWrapping};

/// The box's width.
const WIDTH: f64 = 300.0;

/// A line several times the box's width.
const LONG: &str = "The quick brown fox jumps over the lazy dog while the long line keeps going past the right edge of the box";

struct Fixture {
    cell: Rc<AppCell>,
    /// The box's key, through which a test asks it where its characters are.
    key: GlobalKey,
    clipboard: MockClipboard,
    time: Duration,
    last_mouse: Offset,
}

impl Fixture {
    /// A focused multiline box that does not wrap, `WIDTH` wide, four lines tall, in `direction`.
    fn new(direction: TextDirection, visibility: ScrollBarVisibility) -> Self {
        let clipboard = MockClipboard::new();
        let platform = TestPlatform::new()
            .on(TargetPlatform::Windows)
            .with_view(Rc::new(TestView::new(800.0, 600.0)))
            .with_clipboard(Rc::new(clipboard.clone()))
            .with_font_source(|| Box::new(SystemFontSource::platform()));
        let cell = AppCell::with_platform(Rc::new(platform));
        let key = GlobalKey::new();
        {
            let mut app = cell.borrow_mut();
            PaintingBinding::instance(&mut app).install_platform_fonts(&mut app);
            let controller = TextEditingController::new(&mut app).as_controller();
            let text_box = TextBox::new(controller)
                .key(Rc::new(key.clone()))
                .accepts_return(true)
                .text_wrapping(TextWrapping::NoWrap)
                .horizontal_scroll_bar_visibility(visibility)
                .max_lines(4);
            let home = Overlay::wrap(Directionality::new(
                direction,
                Align::new()
                    .alignment(AlignmentGeometry::CENTER)
                    .child(SizedBox::new().width(WIDTH).child(text_box)),
            ))
            .into_widget();
            let root = WidgetsApp::new(AccentPalette::default().base)
                .builder(move |_, _, _| home.clone())
                .into_widget();
            run_app(&mut app, root);
        }
        cell.elapse(Duration::ZERO);
        let mut fixture = Self {
            cell,
            key,
            clipboard,
            time: Duration::ZERO,
            last_mouse: Offset::ZERO,
        };
        fixture.pump();
        fixture
    }

    fn pump(&mut self) {
        let (cell, views) = (&self.cell, [IMPLICIT_VIEW_ID]);
        for _ in 0..10 {
            self.time += Duration::from_millis(20);
            SchedulerBinding::handle_begin_frame(&mut cell.borrow_mut(), &views, Some(self.time));
            cell.checkpoint();
            SchedulerBinding::handle_draw_frame(&mut cell.borrow_mut(), &views);
            cell.checkpoint();
        }
    }

    fn focus(&mut self) {
        let node = self.find(|app, element| {
            downcast_widget::<EditableText>(element.widget(app).as_ref()).map(|w| w.focus_node)
        });
        node.as_node()
            .request_focus(&mut self.cell.borrow_mut(), None);
        self.pump();
    }

    /// The host IME replacing the text with `text` and the caret at its end, as typing does.
    fn type_text(&mut self, text: &str) {
        let caret = text.encode_utf16().count() as i32;
        let value = TextEditingValue::new()
            .text(text)
            .selection(TextSelection::collapsed(caret, TextAffinity::Downstream));
        let mut app = self.cell.borrow_mut();
        TextInput::instance(&mut app).update_editing_value(&mut app, value);
        drop(app);
        self.cell.checkpoint();
        self.pump();
    }

    fn send_key(&mut self, event: KeyEvent) {
        let mut app = self.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
        drop(app);
        self.cell.checkpoint();
    }

    /// `logical` pressed and released, with `held` down around it.
    fn key(&mut self, held: &[(Physical, Key)], physical: Physical, logical: Key) {
        for (p, l) in held {
            self.send_key(KeyEvent::Down(KeyDownEvent::new(*p, *l, self.time)));
        }
        self.send_key(KeyEvent::Down(KeyDownEvent::new(
            physical, logical, self.time,
        )));
        self.send_key(KeyEvent::Up(KeyUpEvent::new(physical, logical, self.time)));
        for (p, l) in held.iter().rev() {
            self.send_key(KeyEvent::Up(KeyUpEvent::new(*p, *l, self.time)));
        }
        self.pump();
    }

    /// A mouse event, as the host sends it; the button held is the primary one.
    fn mouse(&mut self, change: PointerChange, point: Offset, buttons: i64) {
        let mut app = self.cell.borrow_mut();
        let event = match change {
            PointerChange::Down => PointerEvent::Down(PointerDownEvent {
                pointer: 1,
                position: point,
                kind: PointerDeviceKind::Mouse,
                buttons,
                time_stamp: self.time,
                ..PointerDownEvent::default()
            }),
            PointerChange::Move => PointerEvent::Move(PointerMoveEvent {
                pointer: 1,
                position: point,
                delta: point - self.last_mouse,
                kind: PointerDeviceKind::Mouse,
                buttons,
                down: true,
                time_stamp: self.time,
                ..PointerMoveEvent::default()
            }),
            PointerChange::Up => PointerEvent::Up(PointerUpEvent {
                pointer: 1,
                position: point,
                kind: PointerDeviceKind::Mouse,
                time_stamp: self.time,
                ..PointerUpEvent::default()
            }),
            _ => return,
        };
        GestureBinding::instance(&mut app).handle_pointer_event(&mut app, event);
        app.drain_microtasks();
        drop(app);
        self.last_mouse = point;
        self.cell.checkpoint();
    }

    fn find<T>(&self, pick: impl Fn(&App, AnyElement) -> Option<T>) -> T {
        let mut app = self.cell.borrow_mut();
        let mut stack = Vec::from_iter(WidgetsBinding::instance(&mut app).root_element(&app));
        while let Some(element) = stack.pop() {
            if let Some(found) = pick(&app, element) {
                return found;
            }
            stack.extend(element.children(&app));
        }
        panic!("no such element")
    }

    fn editable(&self) -> RenderHandle<RenderEditable> {
        self.find(|app, element| element.render_object(app)?.downcast::<RenderEditable>(app))
    }

    fn editable_state(&self) -> Handle<EditableTextState> {
        self.find(|app, element| element.state_handle::<EditableTextState>(app))
    }

    /// Where the whole control is on screen.
    fn box_rect(&self) -> Rect {
        self.find(|app, element| {
            downcast_widget::<TextBox>(element.widget(app).as_ref())?;
            let render = element.find_render_object(app)?.as_box()?;
            Some(render.local_to_global(app, Offset::ZERO, None) & render.size(app))
        })
    }

    /// Where the caret is on screen.
    fn caret_rect(&self) -> Rect {
        let editable = self.editable();
        let mut app = self.cell.borrow_mut();
        let extent = editable.selection(&app).unwrap().extent();
        let local = editable.get_local_rect_for_caret(&mut app, extent);
        let origin = editable.as_box().local_to_global(&app, Offset::ZERO, None);
        local.shift(origin)
    }

    /// How far the box's lines are scrolled sideways.
    fn sideways(&self) -> f64 {
        self.find(|app, element| {
            let render = element.render_object(app)?;
            render.downcast::<RenderEditable>(app)?;
            let inner = element.find_ancestor_state_of_type::<ScrollableState>(app)?;
            let outer = inner
                .context(app)
                .find_ancestor_state_of_type::<ScrollableState>(app)?;
            Some(outer.position(app).pixels(app))
        })
    }

    fn assert_caret_in_view(&self, what: &str) {
        let (caret, area) = (self.caret_rect(), self.box_rect());
        assert!(
            caret.left >= area.left - 0.5 && caret.right <= area.right + 0.5,
            "{what}: the caret {caret:?} is across the box {area:?}"
        );
        assert!(
            caret.top >= area.top - 0.5 && caret.bottom <= area.bottom + 0.5,
            "{what}: the caret {caret:?} is up and down the box {area:?}"
        );
    }

    /// `GetRectFromCharacterIndex` of the box.
    fn rect_of(&self, index: i32, trailing_edge: bool) -> Option<Rect> {
        let mut app = self.cell.borrow_mut();
        let state = self.key.current_state::<TextBoxState>(&mut app).unwrap();
        state.get_rect_from_character_index(&mut app, index, trailing_edge)
    }

    fn text(&self) -> String {
        let state = self.editable_state();
        let app = self.cell.borrow();
        state.text_editing_value(&app).text
    }
}

#[test]
fn a_short_text_fills_the_box_and_a_tap_anywhere_lands_in_it() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    let area = f.box_rect();
    let editable = f.editable();
    assert!(
        editable.size(&f.cell.borrow()).width() >= WIDTH - 40.0,
        "the empty editor spans the box"
    );
    let right = Offset::new(area.right - 6.0, area.top + area.height() / 2.0);
    f.mouse(PointerChange::Add, right, 0);
    f.mouse(PointerChange::Down, right, 1);
    f.mouse(PointerChange::Up, right, 0);
    f.pump();
    assert!(
        {
            let node = f.editable_state().widget(&f.cell.borrow()).focus_node;
            node.has_focus(&mut f.cell.borrow_mut())
        },
        "a tap at the box's right edge focuses it"
    );
    f.type_text("ab");
    f.mouse(PointerChange::Down, right, 1);
    f.mouse(PointerChange::Up, right, 0);
    f.pump();
    assert_eq!(
        f.editable()
            .selection(&f.cell.borrow())
            .unwrap()
            .extent()
            .offset,
        2,
        "a tap right of a short line puts the caret at its end"
    );
    assert_eq!(f.sideways(), 0.0);
}

#[test]
fn lines_stay_whole_and_the_caret_stays_in_view_while_typing_past_the_edge() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    f.focus();
    f.type_text(&format!("first line\n{LONG}"));
    let one_line = f.editable().preferred_line_height(&mut f.cell.borrow_mut());
    let height = f.editable().size(&f.cell.borrow()).height();
    assert!(
        height < 3.0 * one_line,
        "two lines, the long one unbroken: {height}"
    );
    assert!(f.sideways() > 0.0, "the box scrolled sideways to the caret");
    f.assert_caret_in_view("typing");
}

#[test]
fn arrow_keys_home_and_end_keep_the_caret_in_view() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    f.focus();
    f.type_text(&format!("{LONG}\n{LONG}"));
    f.key(&[], Physical::HOME, Key::HOME);
    f.assert_caret_in_view("Home");
    assert_eq!(f.sideways(), 0.0, "Home scrolls back to the start");
    f.key(&[], Physical::END, Key::END);
    f.assert_caret_in_view("End");
    let at_end = f.sideways();
    assert!(at_end > 0.0);
    for _ in 0..30 {
        f.key(&[], Physical::ARROW_LEFT, Key::ARROW_LEFT);
    }
    f.assert_caret_in_view("Left");
    f.key(&[], Physical::ARROW_UP, Key::ARROW_UP);
    f.assert_caret_in_view("Up");
    f.key(&[], Physical::HOME, Key::HOME);
    f.key(&[], Physical::ARROW_DOWN, Key::ARROW_DOWN);
    f.assert_caret_in_view("Down");
}

#[test]
fn drag_selecting_past_either_edge_scrolls_the_lines() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    f.focus();
    f.type_text(LONG);
    f.key(&[], Physical::HOME, Key::HOME);
    let area = f.box_rect();
    let row = f.caret_rect().center().dy();
    let start = Offset::new(area.left + 20.0, row);
    f.mouse(PointerChange::Add, start, 0);
    f.mouse(PointerChange::Down, start, 1);
    for step in 1..=12 {
        f.time += Duration::from_millis(30);
        f.mouse(
            PointerChange::Move,
            Offset::new(area.right + 10.0 * step as f64, row),
            1,
        );
        f.pump();
    }
    let selection = f.editable().selection(&f.cell.borrow()).unwrap();
    assert!(!selection.is_collapsed());
    assert!(f.sideways() > 0.0, "dragging past the right edge scrolls");
    f.assert_caret_in_view("dragging right");
    for step in 1..=12 {
        f.time += Duration::from_millis(30);
        f.mouse(
            PointerChange::Move,
            Offset::new(area.left - 10.0 * step as f64, row),
            1,
        );
        f.pump();
    }
    f.mouse(PointerChange::Up, Offset::new(area.left - 120.0, row), 0);
    f.pump();
    f.assert_caret_in_view("dragging left");
}

#[test]
fn the_selection_ends_and_the_menu_sit_where_the_text_is_when_scrolled_sideways() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    f.focus();
    f.type_text(LONG);
    // The last word, selected at the far end of the line.
    f.key(
        &[
            (Physical::SHIFT_LEFT, Key::SHIFT_LEFT),
            (Physical::CONTROL_LEFT, Key::CONTROL_LEFT),
        ],
        Physical::ARROW_LEFT,
        Key::ARROW_LEFT,
    );
    assert!(f.sideways() > 0.0);
    let editable = f.editable();
    let selection = editable.selection(&f.cell.borrow()).unwrap();
    assert!(!selection.is_collapsed());
    let area = f.box_rect();
    let points = {
        let mut app = f.cell.borrow_mut();
        let origin = editable.as_box().local_to_global(&app, Offset::ZERO, None);
        editable
            .get_endpoints_for_selection(&mut app, selection)
            .into_iter()
            .map(|point| point.point + origin)
            .collect::<Vec<_>>()
    };
    for point in &points {
        assert!(
            point.dx() >= area.left && point.dx() <= area.right,
            "a selection end at {point:?} is on the box {area:?}, where the text shows"
        );
    }
    let state = f.editable_state();
    let anchors = state.context_menu_anchors(&mut f.cell.borrow_mut());
    assert!(
        anchors.primary_anchor.dx() >= area.left && anchors.primary_anchor.dx() <= area.right,
        "the menu's anchor {:?} is on the box {area:?}",
        anchors.primary_anchor
    );
}

#[test]
fn a_right_to_left_paragraph_keeps_its_caret_in_view() {
    let mut f = Fixture::new(TextDirection::Rtl, ScrollBarVisibility::Hidden);
    f.focus();
    let line = "שלום עולם ".repeat(12);
    f.type_text(&format!("{line}\nשורה"));
    f.assert_caret_in_view("typing right to left");
    f.key(&[], Physical::ARROW_UP, Key::ARROW_UP);
    f.key(&[], Physical::END, Key::END);
    f.assert_caret_in_view("End of the long line");
    f.key(&[], Physical::HOME, Key::HOME);
    f.assert_caret_in_view("Home of the long line");
}

#[test]
fn pasting_a_very_long_line_scrolls_to_its_end() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Auto);
    f.focus();
    let pasted = "0123456789 ".repeat(400);
    f.clipboard.set_clipboard_data(Some(pasted.clone()));
    f.key(
        &[(Physical::CONTROL_LEFT, Key::CONTROL_LEFT)],
        Physical::KEY_V,
        Key::KEY_V,
    );
    f.pump();
    assert_eq!(f.text(), pasted);
    let one_line = f.editable().preferred_line_height(&mut f.cell.borrow_mut());
    assert!(
        f.editable().size(&f.cell.borrow()).height() < 2.0 * one_line,
        "one line"
    );
    assert!(f.sideways() > 1000.0);
    f.assert_caret_in_view("after the paste");
}

#[test]
fn disabled_scrolling_clips_the_lines_at_the_box() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Disabled);
    f.focus();
    f.type_text(LONG);
    let editable = f.editable();
    // The line runs past the box, which shows only its start.
    assert!(editable.size(&f.cell.borrow()).width() > WIDTH);
    let area = f.box_rect();
    let left = editable
        .as_box()
        .local_to_global(&f.cell.borrow(), Offset::ZERO, None)
        .dx();
    assert!(
        left > area.left && left < area.left + 20.0,
        "the line starts at the box's padding, not scrolled: {left} in {area:?}"
    );
    let one_line = editable.preferred_line_height(&mut f.cell.borrow_mut());
    let height = editable.size(&f.cell.borrow()).height();
    assert!(
        height < 2.0 * one_line,
        "unbroken: {height} for lines of {one_line}"
    );
}

// MARK: GetRectFromCharacterIndex

#[test]
fn an_empty_box_and_indices_outside_the_text_have_no_rectangle() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    assert_eq!(f.rect_of(0, false), None, "an empty box");
    f.focus();
    f.type_text("ab");
    assert_eq!(f.rect_of(-1, false), None);
    assert_eq!(
        f.rect_of(2, false),
        None,
        "the end of the text is no character"
    );
    assert!(f.rect_of(1, true).is_some());
}

#[test]
fn a_characters_edges_are_in_the_boxs_own_coordinates() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    f.focus();
    f.type_text("ab\ncd");
    let a = f.rect_of(0, false).unwrap();
    let a_end = f.rect_of(0, true).unwrap();
    let b = f.rect_of(1, false).unwrap();
    let c = f.rect_of(3, false).unwrap();
    assert_eq!(a.width(), 0.0);
    assert!(a.height() > 0.0);
    assert!(
        a.left > 0.0 && a.left < 20.0,
        "inside the box's border and padding: {a:?}"
    );
    assert!(
        a_end.left > a.left,
        "the trailing edge is right of the leading one"
    );
    assert!(
        (a_end.left - b.left).abs() < 0.5,
        "and is the next character's leading edge"
    );
    assert!(
        (c.left - a.left).abs() < 0.5,
        "the second line starts where the first does"
    );
    assert!((c.top - (a.top + a.height())).abs() < 1.0, "a line lower");
    // The same point on screen as the caret before the character.
    let area = f.box_rect();
    f.key(&[], Physical::HOME, Key::HOME);
    let caret = f.caret_rect();
    assert!(
        (area.left + c.left - caret.left).abs() < 1.0,
        "{c:?} against {caret:?}"
    );
}

#[test]
fn the_rectangle_follows_scrolling_on_both_axes() {
    let mut f = Fixture::new(TextDirection::Ltr, ScrollBarVisibility::Hidden);
    f.focus();
    let text = format!("{LONG}\n1\n2\n3\n4\n5\n6\nend");
    f.type_text(&text);
    let last = text.encode_utf16().count() as i32 - 1;
    // The caret is at the end: the box scrolled down; its first line is above it.
    let first = f.rect_of(0, false).unwrap();
    assert!(
        first.top < 0.0,
        "the first line scrolled up out of the box: {first:?}"
    );
    let end = f.rect_of(last, true).unwrap();
    let area = f.box_rect();
    assert!(end.top >= 0.0 && end.bottom <= area.height(), "{end:?}");
    // At the end of the long line the box scrolled sideways too.
    f.key(
        &[(Physical::CONTROL_LEFT, Key::CONTROL_LEFT)],
        Physical::HOME,
        Key::HOME,
    );
    f.key(&[], Physical::END, Key::END);
    let long_end = LONG.encode_utf16().count() as i32 - 1;
    let start = f.rect_of(0, false).unwrap();
    let tail = f.rect_of(long_end, true).unwrap();
    assert!(
        start.left < 0.0,
        "the line's start scrolled out at the left: {start:?}"
    );
    assert!(
        tail.left > 0.0 && tail.left <= WIDTH,
        "its end shows: {tail:?}"
    );
}

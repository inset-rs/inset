//! AutoSuggestBox: TextChanged's reasons and its 150 ms delay, the list opening as suggestions arrive for typed text, the arrow keys moving through it and showing the suggestion moved to, Enter, a click and the query button submitting, Escape putting back the typed text, the list opening above a box near the window's bottom in reverse order, the list closing as the focus leaves, and the gallery page.

use crate::common::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
use waft_embedder::{Color, Offset, PointerChange, Rect, TextAffinity, TextRange, TextSelection};
use waft_foundation::{App, Handle, Ref};
use waft_rendering::RenderParagraph;
use waft_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical, TextEditingValue, TextInput,
};
use waft_widgets::*;
use waft_winui::*;

const WORDS: [&str; 6] = ["Alpha", "Almond", "Alpine", "Beta", "Bravo", "Gamma"];

/// What the test and the box's owner share.
#[derive(Default)]
struct Shared {
    /// The box's top in the window.
    top: Cell<f64>,
    controller: RefCell<Option<TextEditingControllerRef>>,
    items: RefCell<Vec<String>>,
    events: RefCell<Vec<String>>,
    /// The arguments of every TextChanged, to check `CheckCurrent` after the fact.
    changes: RefCell<Vec<AutoSuggestBoxTextChangedEventArgs>>,
    host: Cell<Option<Handle<HostState>>>,
}

thread_local! {
    static SHARED: RefCell<Option<Rc<Shared>>> = const { RefCell::new(None) };
}

#[derive(Debug)]
struct Host;

struct HostState {
    state: StateData<Host>,
    shared: Rc<Shared>,
    controller: Option<Ref<TextEditingController>>,
    /// The text box after the box, which Tab moves to.
    next: Option<Ref<TextEditingController>>,
}

impl StatefulWidget for Host {
    type State = HostState;

    fn create_state(&self) -> HostState {
        HostState {
            state: StateData::new(),
            shared: SHARED.with(|shared| shared.borrow().clone().unwrap()),
            controller: None,
            next: None,
        }
    }
}

impl State for HostState {
    type Widget = Host;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = TextEditingController::new(app);
        let shared = app.get(self).shared.clone();
        *shared.controller.borrow_mut() = Some(controller.erase());
        shared.host.set(Some(self));
        let next = TextEditingController::new(app);
        let state = app.get_mut(self);
        state.controller = Some(controller);
        state.next = Some(next);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let shared = app.get(self).shared.clone();
        let controller = shared.controller.borrow().clone().unwrap();
        let next = app.get(self).next.as_ref().unwrap().erase();
        let (changed, chosen, submitted) = (shared.clone(), shared.clone(), shared.clone());
        let suggest = AutoSuggestBox::new(controller.clone())
            .items_source(shared.items.borrow().clone())
            .placeholder_text("Search")
            .query_icon(FontIcon::symbol(FluentSymbol::Search).font_size(12.0))
            .text_changed(move |app, args| {
                let text = controller.text_value(app).to_owned();
                changed
                    .events
                    .borrow_mut()
                    .push(format!("changed {:?} {text}", args.reason()));
                changed.changes.borrow_mut().push(*args);
                if args.reason() == AutoSuggestionBoxTextChangeReason::UserInput {
                    *changed.items.borrow_mut() = WORDS
                        .iter()
                        .filter(|word| !text.is_empty() && word.starts_with(&text))
                        .map(|word| word.to_string())
                        .collect();
                    self.set_state(app, |_| {});
                }
            })
            .suggestion_chosen(move |_, args| {
                chosen
                    .events
                    .borrow_mut()
                    .push(format!("chosen {}", args.selected_item()));
            })
            .query_submitted(move |_, args| {
                submitted.events.borrow_mut().push(format!(
                    "submitted {} {:?}",
                    args.query_text(),
                    args.chosen_suggestion()
                ));
            });
        Padding::new(waft_painting::EdgeInsetsGeometry::only(
            20.0,
            shared.top.get(),
            0.0,
            0.0,
        ))
        .child(
            Align::new()
                .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
                .child(
                    Column::new()
                        .main_axis_size(waft_rendering::MainAxisSize::Min)
                        .cross_axis_alignment(waft_rendering::CrossAxisAlignment::Start)
                        .spacing(8.0)
                        .children([
                            SizedBox::new().width(300.0).child(suggest).into_widget(),
                            SizedBox::new()
                                .width(300.0)
                                .child(TextBox::new(next))
                                .into_widget(),
                        ]),
                ),
        )
        .into_widget()
    }
}

fn fixture(top: f64) -> (Fixture, Rc<Shared>) {
    let shared = Rc::new(Shared::default());
    shared.top.set(top);
    SHARED.with(|slot| *slot.borrow_mut() = Some(shared.clone()));
    let mut f = Fixture::with_root([800, 600], move |app| {
        winui_gallery::install_fonts(app);
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
                    route.into_erased()
                })
                .home(ThemeScope::new(
                    Theme::Light,
                    ColoredBox::new(Color::from_argb(255, 255, 255, 255)).child(Host),
                ))
                .into_widget(),
        );
    });
    settle(&mut f);
    (f, shared)
}

/// Lets `duration` pass on the App clock in 20 ms steps, with a frame after each (the timers
/// fire from `AppCell::elapse`, not from frames).
fn pass(f: &mut Fixture, duration: Duration) {
    let step = Duration::from_millis(20);
    let mut elapsed = Duration::ZERO;
    while elapsed < duration {
        f.cell.elapse(step);
        f.frame_after(step);
        elapsed += step;
    }
}

/// Lets 400 ms pass: longer than TextChanged's delay.
fn settle(f: &mut Fixture) {
    pass(f, Duration::from_millis(400));
}

/// The centre of the box's text.
fn text_box(shared: &Shared) -> Offset {
    Offset::new(120.0, shared.top.get() + 16.0)
}

fn tap_at(f: &mut Fixture, point: Offset) {
    f.send(PointerChange::Down, point);
    f.at += Duration::from_millis(20);
    f.send(PointerChange::Up, point);
    // Past the double-tap timeout, so that the next tap is a tap of its own.
    settle(f);
}

/// The editing value a host's input method sends: `text` with the caret at its end.
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
    }
    settle(f);
}

fn text(f: &Fixture, shared: &Shared) -> String {
    let controller = shared.controller.borrow().clone().unwrap();
    controller.text_value(&f.cell.borrow()).to_owned()
}

fn events(shared: &Shared) -> Vec<String> {
    shared.events.borrow_mut().drain(..).collect()
}

/// The global rectangle of each label reading `text`.
fn labels(f: &Fixture, text: &str) -> Vec<Rect> {
    let elements = f.elements();
    let app = f.cell.borrow();
    let mut found = Vec::new();
    for element in elements {
        if let Some(object) = element.render_object(&app)
            && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
            && paragraph.text(&app).to_plain_text(true, true) == text
        {
            let object = object.as_box().unwrap();
            let origin = object.local_to_global(&app, Offset::ZERO, None);
            let rect = origin & object.size(&app);
            if !found.contains(&rect) {
                found.push(rect);
            }
        }
    }
    found
}

/// The suggestions the open list shows, top to bottom.
fn suggestions(f: &Fixture) -> Vec<&'static str> {
    let mut shown: Vec<(f64, &'static str)> = WORDS
        .iter()
        .flat_map(|&word| {
            labels(f, word)
                .into_iter()
                .map(move |rect| (rect.top, word))
        })
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0));
    shown.into_iter().map(|(_, word)| word).collect()
}

/// Focuses the box and types `typed`, waiting for TextChanged and the suggestions it brings.
fn type_into_box(f: &mut Fixture, shared: &Shared, typed: &str) {
    tap_at(f, text_box(shared));
    type_text(f, typed);
    settle(f);
}

#[test]
fn text_changed_comes_150_ms_after_the_last_change_with_its_reason() {
    let (mut f, shared) = fixture(40.0);
    tap_at(&mut f, text_box(&shared));
    type_text(&mut f, "A");
    pass(&mut f, Duration::from_millis(100));
    type_text(&mut f, "Al");
    pass(&mut f, Duration::from_millis(100));
    assert!(events(&shared).is_empty(), "no TextChanged before 150 ms");
    pass(&mut f, Duration::from_millis(60));
    assert_eq!(events(&shared), ["changed UserInput Al"]);
    assert!(shared.changes.borrow()[0].check_current(&f.cell.borrow()));

    let controller = shared.controller.borrow().clone().unwrap();
    controller.set_text(&mut f.cell.borrow_mut(), "Beta");
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(events(&shared), ["changed ProgrammaticChange Beta"]);
    assert!(!shared.changes.borrow()[0].check_current(&f.cell.borrow()));
    assert!(shared.changes.borrow()[1].check_current(&f.cell.borrow()));
}

#[test]
fn suggestions_for_typed_text_open_the_list_below_the_box() {
    let (mut f, shared) = fixture(40.0);
    type_into_box(&mut f, &shared, "Al");
    assert_eq!(suggestions(&f), ["Alpha", "Almond", "Alpine"]);
    // The list's top meets the text box's bottom.
    assert!(labels(&f, "Alpha")[0].top > 40.0 + 32.0);
    f.capture("auto_suggest_box_below");

    type_text(&mut f, "");
    settle(&mut f);
    assert!(suggestions(&f).is_empty(), "no suggestions closes the list");
}

#[test]
fn arrow_keys_move_through_the_list_and_show_the_suggestion_moved_to() {
    let (mut f, shared) = fixture(40.0);
    type_into_box(&mut f, &shared, "Al");
    events(&shared);

    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(text(&f, &shared), "Alpha");
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(text(&f, &shared), "Almond");
    assert_eq!(
        events(&shared),
        [
            "chosen Alpha",
            "changed SuggestionChosen Alpha",
            "chosen Almond",
            "changed SuggestionChosen Almond"
        ]
    );
    // Up from the first suggestion goes back to the box and the typed text.
    key(&mut f, Physical::ARROW_UP, Key::ARROW_UP);
    key(&mut f, Physical::ARROW_UP, Key::ARROW_UP);
    assert_eq!(text(&f, &shared), "Al");
    assert_eq!(
        events(&shared),
        [
            "chosen Alpha",
            "changed SuggestionChosen Alpha",
            "changed ProgrammaticChange Al"
        ]
    );
    // Up from the box goes to the last suggestion.
    key(&mut f, Physical::ARROW_UP, Key::ARROW_UP);
    assert_eq!(text(&f, &shared), "Alpine");
    assert_eq!(
        suggestions(&f),
        ["Alpha", "Almond", "Alpine"],
        "the list stays open"
    );
}

#[test]
fn enter_submits_the_selected_suggestion_or_the_text() {
    let (mut f, shared) = fixture(40.0);
    type_into_box(&mut f, &shared, "Al");
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    events(&shared);
    key(&mut f, Physical::ENTER, Key::ENTER);
    assert_eq!(events(&shared), ["submitted Alpha Some(\"Alpha\")"]);
    assert!(suggestions(&f).is_empty(), "submitting closes the list");

    type_text(&mut f, "Be");
    settle(&mut f);
    events(&shared);
    key(&mut f, Physical::ENTER, Key::ENTER);
    assert_eq!(events(&shared), ["submitted Be None"]);
}

#[test]
fn a_click_chooses_the_suggestion_then_submits_it() {
    let (mut f, shared) = fixture(40.0);
    type_into_box(&mut f, &shared, "Al");
    events(&shared);
    let almond = labels(&f, "Almond")[0];
    tap_at(&mut f, almond.center());
    assert_eq!(text(&f, &shared), "Almond");
    let events = events(&shared);
    assert_eq!(
        &events[..2],
        ["chosen Almond", "submitted Almond Some(\"Almond\")"]
    );
    assert!(suggestions(&f).is_empty());
}

#[test]
fn the_query_button_submits_the_text_alone() {
    let (mut f, shared) = fixture(40.0);
    type_into_box(&mut f, &shared, "Al");
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    events(&shared);
    // `QueryButton` is the 32 pixels before the template's last 4.
    tap_at(&mut f, Offset::new(20.0 + 300.0 - 4.0 - 16.0, 40.0 + 16.0));
    assert_eq!(events(&shared), ["submitted Alpha None"]);
}

#[test]
fn escape_puts_back_the_typed_text_and_closes_the_list() {
    let (mut f, shared) = fixture(40.0);
    type_into_box(&mut f, &shared, "Al");
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(text(&f, &shared), "Alpha");
    key(&mut f, Physical::ESCAPE, Key::ESCAPE);
    assert_eq!(text(&f, &shared), "Al");
    assert!(suggestions(&f).is_empty());
}

#[test]
fn near_the_bottom_the_list_opens_above_with_the_first_suggestion_next_to_the_box() {
    let (mut f, shared) = fixture(480.0);
    type_into_box(&mut f, &shared, "Al");
    assert_eq!(suggestions(&f), ["Alpine", "Almond", "Alpha"]);
    assert!(labels(&f, "Alpha")[0].bottom < 480.0);
    f.capture("auto_suggest_box_above");
    // Up moves from the box to the suggestion next to it.
    key(&mut f, Physical::ARROW_UP, Key::ARROW_UP);
    assert_eq!(text(&f, &shared), "Alpha");
}

#[test]
fn the_list_closes_when_the_focus_leaves_the_box() {
    let (mut f, shared) = fixture(40.0);
    type_into_box(&mut f, &shared, "Al");
    assert!(!suggestions(&f).is_empty());
    key(&mut f, Physical::TAB, Key::TAB);
    assert!(suggestions(&f).is_empty());
    assert_eq!(text(&f, &shared), "Al");
}

#[test]
fn the_gallery_page_suggests_colors() {
    use crate::common::GalleryFixtureExt;
    let mut f = Fixture::for_feature([1080, 900], winui_gallery::Feature::AutoSuggestBox);
    f.capture("gallery_auto_suggest_box_light");
    f.tap("Search colors");
    type_text(&mut f, "B");
    settle(&mut f);
    f.find("Bronze");
    f.capture("gallery_auto_suggest_box_open_light");
}

//! TableView: the header and every row sharing each column's width, resizing a column from its gripper, frozen columns staying put while the rest scroll, the sort cycle and a canceled sort, filtering, the selection by pointer, keyboard and code and following its item through a sort, and the empty state.

use crate::common::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
use waft_embedder::{
    Color, Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind,
    PointerSignalKind, Rect,
};
use waft_foundation::Ref;
use waft_gestures::GestureBinding;
use waft_rendering::RenderParagraph;
use waft_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical,
};
use waft_widgets::*;
use waft_winui::*;

#[derive(Clone, Debug)]
struct Person {
    name: &'static str,
    role: &'static str,
    city: &'static str,
    age: u32,
}

fn people() -> Vec<Person> {
    vec![
        Person {
            name: "Mina",
            role: "Program Manager",
            city: "Tokyo",
            age: 41,
        },
        Person {
            name: "Asha",
            role: "Designer",
            city: "Bengaluru",
            age: 29,
        },
        Person {
            name: "Diego",
            role: "Engineer",
            city: "Madrid",
            age: 35,
        },
        Person {
            name: "Lea",
            role: "Researcher",
            city: "Lyon",
            age: 29,
        },
    ]
}

/// What a test holds of its table.
struct Table {
    source: Ref<TableViewSource<Person>>,
    name: Ref<TableViewColumn<Person>>,
    role: Ref<TableViewColumn<Person>>,
    city: Ref<TableViewColumn<Person>>,
    age: Ref<TableViewColumn<Person>>,
    selection: Ref<TableViewSelection>,
    sort: Ref<TableViewSort>,
    sorted: Cell<usize>,
    cancel_sorting: Cell<bool>,
    selection_changes: RefCell<Vec<TableViewSelectionChangedEventArgs>>,
}

/// How the table under test is set up.
#[derive(Clone, Copy)]
struct Setup {
    width: f64,
    items: bool,
    frozen: bool,
}

impl Default for Setup {
    fn default() -> Self {
        Setup {
            width: 560.0,
            items: true,
            frozen: false,
        }
    }
}

fn fixture(setup: Setup) -> (Fixture, Rc<Table>) {
    let shared: Rc<RefCell<Option<Rc<Table>>>> = Rc::default();
    let slot = shared.clone();
    let f = Fixture::with_root([640, 400], move |app| {
        winui_gallery::install_fonts(app);
        let source = TableViewSource::from(app, if setup.items { people() } else { Vec::new() });
        let name = TableViewColumn::text(app, "Name", |p: &Person| p.name.to_string());
        name.set_width(app, GridLength::pixel(150.0));
        if setup.frozen {
            name.set_frozen_edge(app, TableViewFrozenEdge::Leading);
        }
        let role = TableViewColumn::text(app, "Role", |p: &Person| p.role.to_string());
        role.set_width(app, GridLength::AUTO);
        let city = TableViewColumn::text(app, "City", |p: &Person| p.city.to_string());
        city.set_width(app, GridLength::STAR);
        city.set_min_width(app, 100.0);
        let age = TableViewColumn::text(app, "Age", |p: &Person| p.age.to_string());
        age.set_width(app, GridLength::pixel(80.0));
        age.set_sort_key(app, |p: &Person| p.age);
        age.set_sort_cycle(app, TableViewSortCycle::DescendingAscendingNone);
        let table = Rc::new(Table {
            source: source.clone(),
            name: name.clone(),
            role: role.clone(),
            city: city.clone(),
            age: age.clone(),
            selection: TableViewSelection::new(app),
            sort: TableViewSort::new(app),
            sorted: Cell::new(0),
            cancel_sorting: Cell::new(false),
            selection_changes: RefCell::default(),
        });
        *slot.borrow_mut() = Some(table.clone());
        let view = TableView::new(source, vec![name, role, city, age])
            .selection(table.selection.clone())
            .sort(table.sort.clone())
            .empty_template(|_, _| Text::new("No people").into_widget())
            .sorting({
                let table = table.clone();
                move |_, args| args.set_cancel(table.cancel_sorting.get())
            })
            .sorted({
                let table = table.clone();
                move |_, _| table.sorted.set(table.sorted.get() + 1)
            })
            .selection_changed({
                let table = table.clone();
                move |_, args| table.selection_changes.borrow_mut().push(args.clone())
            });
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    ThemeScope::new(
                        Theme::Light,
                        ColoredBox::new(Color::from_argb(255, 255, 255, 255)).child(
                            Align::new()
                                .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
                                .child(
                                    SizedBox::new()
                                        .width(setup.width)
                                        .height(300.0)
                                        .child(view.clone()),
                                ),
                        ),
                    )
                    .into_widget()
                })
                .into_widget(),
        );
    });
    let table = shared.borrow().clone().unwrap();
    let mut f = f;
    settle(&mut f);
    (f, table)
}

fn settle(f: &mut Fixture) {
    for _ in 0..8 {
        f.frame_after(Duration::from_millis(20));
    }
}

/// The global rectangle of the first onstage label `text`, or `None`.
fn label(f: &Fixture, text: &str) -> Option<Rect> {
    let elements = f.elements();
    let app = f.cell.borrow();
    for element in elements {
        if let Some(object) = element.render_object(&app)
            && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
            && paragraph.text(&app).to_plain_text(true, true) == text
        {
            let object = object.as_box().unwrap();
            let origin = object.local_to_global(&app, Offset::ZERO, None);
            return Some(origin & object.size(&app));
        }
    }
    None
}

fn rect(f: &Fixture, text: &str) -> Rect {
    label(f, text).unwrap_or_else(|| panic!("missing label {text}"))
}

/// The names in the order the rows show them.
fn order(f: &Fixture) -> Vec<&'static str> {
    let mut shown: Vec<(f64, &'static str)> = people()
        .into_iter()
        .filter_map(|person| label(f, person.name).map(|rect| (rect.top, person.name)))
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0));
    shown.into_iter().map(|(_, name)| name).collect()
}

/// A touch tap on the label `text`'s centre.
fn tap_label(f: &mut Fixture, text: &str) {
    let point = rect(f, text).center();
    tap_at(f, point);
}

/// A mouse wheel turned over `point`.
fn wheel(f: &mut Fixture, point: Offset, x: f64, y: f64) {
    let mut app = f.cell.borrow_mut();
    GestureBinding::instance(&mut app).handle_pointer_data_packet(
        &mut app,
        PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Hover,
            kind: PointerDeviceKind::Mouse,
            signal_kind: Some(PointerSignalKind::Scroll),
            physical_x: point.dx(),
            physical_y: point.dy(),
            scroll_delta_x: x,
            scroll_delta_y: y,
            ..Default::default()
        }]),
    );
    drop(app);
    f.cell.checkpoint();
    settle(f);
}

fn tap_at(f: &mut Fixture, point: Offset) {
    f.send(PointerChange::Down, point);
    f.at += Duration::from_millis(20);
    f.send(PointerChange::Up, point);
    settle(f);
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

fn with_control(f: &mut Fixture, action: impl FnOnce(&mut Fixture)) {
    let down = KeyEvent::Down(KeyDownEvent::new(
        Physical::CONTROL_LEFT,
        Key::CONTROL_LEFT,
        f.at,
    ));
    {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &down);
    }
    f.cell.checkpoint();
    action(f);
    let up = KeyEvent::Up(KeyUpEvent::new(
        Physical::CONTROL_LEFT,
        Key::CONTROL_LEFT,
        f.at,
    ));
    {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &up);
    }
    f.cell.checkpoint();
}

fn width(f: &Fixture, column: &Ref<TableViewColumn<Person>>) -> f64 {
    column.actual_width(&f.cell.borrow())
}

// MARK: Layout

#[test]
fn the_header_and_every_row_share_each_columns_width() {
    let (f, t) = fixture(Setup::default());
    assert_eq!(width(&f, &t.name), 150.0);
    assert_eq!(width(&f, &t.age), 80.0);
    // `Auto` takes the widest realized cell, its padding and its grid line included, whole, and
    // the star column the rest of the 558 pixel viewport inside the border.
    let role = width(&f, &t.role);
    let widest = rect(&f, "Program Manager").width();
    assert_eq!(role, (widest + 16.0 + 1.0).ceil(), "widest text {widest}");
    assert_eq!(width(&f, &t.city), (558.0 - 150.0 - role - 80.0).round());
    // Each header label starts where its column's cells start.
    for (header, cell) in [
        ("Name", "Mina"),
        ("Role", "Designer"),
        ("City", "Tokyo"),
        ("Age", "41"),
    ] {
        assert_eq!(rect(&f, header).left, rect(&f, cell).left, "{header}");
    }
    assert_eq!(rect(&f, "Role").left - rect(&f, "Name").left, 150.0);
    f.capture("table-view");
}

#[test]
fn a_star_column_held_at_its_min_width_makes_the_table_scroll_horizontally() {
    let (f, t) = fixture(Setup {
        width: 300.0,
        ..Setup::default()
    });
    assert_eq!(width(&f, &t.city), 100.0);
    assert!(
        label(&f, "41").is_some_and(|rect| rect.left > 300.0),
        "Age lies past the viewport"
    );
}

#[test]
fn dragging_a_gripper_resizes_the_column_and_escape_cancels_a_drag() {
    let (mut f, t) = fixture(Setup::default());
    let header = rect(&f, "Name");
    let edge = Offset::new(1.0 + 150.0 - 3.0, header.center().dy());
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    f.send_mouse(PointerChange::Hover, edge, 0);
    f.send_mouse(PointerChange::Down, edge, 1);
    for step in 1..=4 {
        f.at += Duration::from_millis(16);
        f.send_mouse(
            PointerChange::Move,
            edge + Offset::new(10.0 * step as f64, 0.0),
            1,
        );
    }
    f.send_mouse(PointerChange::Up, edge + Offset::new(40.0, 0.0), 0);
    settle(&mut f);
    let resized = width(&f, &t.name);
    assert!((185.0..=191.0).contains(&resized), "resized to {resized}");
    assert_eq!(t.name.width(&f.cell.borrow()).unit, GridUnitType::Pixel);
    assert_eq!(
        rect(&f, "Role").left - rect(&f, "Name").left,
        resized,
        "the cells follow"
    );
    assert_eq!(t.sorted.get(), 0, "a drag on the gripper does not sort");

    tap_label(&mut f, "Asha");
    let edge = Offset::new(1.0 + resized - 3.0, header.center().dy());
    f.send_mouse(PointerChange::Down, edge, 1);
    for step in 1..=4 {
        f.at += Duration::from_millis(16);
        f.send_mouse(
            PointerChange::Move,
            edge + Offset::new(-10.0 * step as f64, 0.0),
            1,
        );
    }
    settle(&mut f);
    assert!(width(&f, &t.name) < resized);
    // Escape reaches the table while the focus is in it, here on the row tapped before the drag.
    key(&mut f, Physical::ESCAPE, Key::ESCAPE);
    f.send_mouse(PointerChange::Up, edge, 0);
    settle(&mut f);
    assert_eq!(
        width(&f, &t.name),
        resized,
        "a canceled drag restores the width"
    );
}

#[test]
fn frozen_columns_stay_while_the_rest_scroll() {
    let (mut f, _) = fixture(Setup {
        width: 300.0,
        frozen: true,
        ..Setup::default()
    });
    let name = rect(&f, "Mina").left;
    let role = rect(&f, "Program Manager").left;
    let over = Offset::new(250.0, rect(&f, "Asha").center().dy());
    wheel(&mut f, over, 120.0, 0.0);
    settle(&mut f);
    assert_eq!(rect(&f, "Mina").left, name, "the frozen cell is pinned");
    assert_eq!(
        rect(&f, "Name").left,
        rect(&f, "Mina").left,
        "and its header with it"
    );
    let moved = role - rect(&f, "Program Manager").left;
    assert!(moved > 50.0, "the rest scrolled by {moved}");
    assert_eq!(
        rect(&f, "Role").left,
        rect(&f, "Program Manager").left,
        "the header follows the rows"
    );
}

// MARK: Sorting

#[test]
fn a_header_click_walks_the_columns_sort_cycle() {
    let (mut f, t) = fixture(Setup::default());
    assert_eq!(order(&f), ["Mina", "Asha", "Diego", "Lea"]);
    f.tap("Name");
    settle(&mut f);
    assert_eq!(order(&f), ["Asha", "Diego", "Lea", "Mina"]);
    assert_eq!(
        t.name.sort_direction(&f.cell.borrow()),
        SortDirection::Ascending
    );
    f.tap("Name");
    settle(&mut f);
    assert_eq!(order(&f), ["Mina", "Lea", "Diego", "Asha"]);
    // `AscendingDescending` has no unsorted step.
    f.tap("Name");
    settle(&mut f);
    assert_eq!(order(&f), ["Asha", "Diego", "Lea", "Mina"]);
    // Age opens descending and returns to source order; sorting it clears the name's sort, and
    // its ties keep source order.
    f.tap("Age");
    settle(&mut f);
    assert_eq!(t.name.sort_direction(&f.cell.borrow()), SortDirection::None);
    assert_eq!(order(&f), ["Mina", "Diego", "Asha", "Lea"]);
    f.tap("Age");
    settle(&mut f);
    assert_eq!(order(&f), ["Asha", "Lea", "Diego", "Mina"]);
    f.tap("Age");
    settle(&mut f);
    assert_eq!(order(&f), ["Mina", "Asha", "Diego", "Lea"]);
    assert_eq!(t.age.sort_direction(&f.cell.borrow()), SortDirection::None);
    assert_eq!(t.sorted.get(), 6);
}

#[test]
fn a_canceled_sort_leaves_the_rows_and_raises_no_sorted() {
    let (mut f, t) = fixture(Setup::default());
    t.cancel_sorting.set(true);
    f.tap("Name");
    settle(&mut f);
    assert_eq!(order(&f), ["Mina", "Asha", "Diego", "Lea"]);
    assert_eq!(t.name.sort_direction(&f.cell.borrow()), SortDirection::None);
    assert_eq!(t.sorted.get(), 0);
}

#[test]
fn sorting_from_code_goes_through_the_table() {
    let (mut f, t) = fixture(Setup::default());
    let unbound = TableViewSort::new(&mut f.cell.borrow_mut());
    assert!(!unbound.sort_by_column(&mut f.cell.borrow_mut(), &t.city, SortDirection::Ascending));
    assert!(
        t.sort
            .sort_by_column(&mut f.cell.borrow_mut(), &t.city, SortDirection::Descending)
    );
    assert!(
        !t.sort
            .sort_by_column(&mut f.cell.borrow_mut(), &t.city, SortDirection::Descending),
        "already so"
    );
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(order(&f), ["Mina", "Diego", "Lea", "Asha"]);
    assert!(t.sort.clear_sort(&mut f.cell.borrow_mut()));
    assert!(!t.sort.clear_sort(&mut f.cell.borrow_mut()));
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(order(&f), ["Mina", "Asha", "Diego", "Lea"]);
}

// MARK: Filtering

#[test]
fn a_filter_on_the_source_shows_only_what_it_keeps() {
    let (mut f, t) = fixture(Setup::default());
    t.source.filter(&mut f.cell.borrow_mut(), |p| p.age == 29);
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(order(&f), ["Asha", "Lea"]);
    t.source.clear_filter(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(order(&f), ["Mina", "Asha", "Diego", "Lea"]);
}

// MARK: Selection

#[test]
fn a_tap_selects_a_row_and_control_tap_deselects_it() {
    let (mut f, t) = fixture(Setup::default());
    tap_label(&mut f, "Diego");
    assert_eq!(t.selection.selected_index(&f.cell.borrow()), Some(2));
    assert_eq!(t.selection.selected_item(&f.cell.borrow()), Some(2));
    f.capture("table-view-selected");
    tap_label(&mut f, "Lea");
    with_control(&mut f, |f| tap_label(f, "Lea"));
    assert_eq!(t.selection.selected_index(&f.cell.borrow()), None);
    let changes = t.selection_changes.borrow();
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[1].removed_items, [2]);
    assert_eq!(changes[1].added_items, [3]);
    assert_eq!(changes[2].added_items, Vec::<usize>::new());
}

#[test]
fn the_selection_follows_the_arrow_keys_and_its_item_through_a_sort() {
    let (mut f, t) = fixture(Setup::default());
    tap_label(&mut f, "Mina");
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(t.selection.selected_index(&f.cell.borrow()), Some(1));
    key(&mut f, Physical::END, Key::END);
    assert_eq!(t.selection.selected_item(&f.cell.borrow()), Some(3), "Lea");
    f.tap("Name");
    settle(&mut f);
    assert_eq!(order(&f), ["Asha", "Diego", "Lea", "Mina"]);
    assert_eq!(
        t.selection.selected_index(&f.cell.borrow()),
        Some(2),
        "Lea's new row"
    );
    assert_eq!(t.selection.selected_item(&f.cell.borrow()), Some(3));
}

#[test]
fn new_items_clear_the_selection() {
    let (mut f, t) = fixture(Setup::default());
    tap_label(&mut f, "Asha");
    t.source.set_items(&mut f.cell.borrow_mut(), people());
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(t.selection.selected_index(&f.cell.borrow()), None);
}

// MARK: Empty state

#[test]
fn an_empty_table_shows_its_empty_template() {
    let (mut f, t) = fixture(Setup {
        items: false,
        ..Setup::default()
    });
    assert!(label(&f, "No people").is_some());
    assert!(label(&f, "Name").is_some(), "the header stays");
    t.source.set_items(&mut f.cell.borrow_mut(), people());
    f.cell.checkpoint();
    settle(&mut f);
    assert!(label(&f, "No people").is_none());
    assert!(label(&f, "Mina").is_some());
}

// MARK: Gallery

#[test]
fn the_gallery_page_shows_a_table_and_an_empty_one() {
    use crate::common::GalleryFixtureExt;
    let mut f = Fixture::for_feature([1000, 800], winui_gallery::Feature::TableView);
    settle(&mut f);
    assert!(label(&f, "browser").is_some());
    assert!(label(&f, "No connections yet").is_some());
    f.tap("Largest first");
    settle(&mut f);
    let backup = rect(&f, "backup").top;
    assert!(
        backup < rect(&f, "music").top && backup < rect(&f, "browser").top,
        "the largest on top"
    );
    f.capture("table-view-gallery");
}

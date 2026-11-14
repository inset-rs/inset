//! ItemsView and ItemContainer: each selection mode's rules as `ItemsView`'s selectors apply them, the pointer states and the selection visual, the arrow keys and Home, End and the pages, focus and Tab, invoking, and a long list staying lazy.

use crate::common::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
use waft_embedder::{Color, Offset, PointerChange};
use waft_foundation::Handle;
use waft_painting::EdgeInsetsGeometry;
use waft_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical,
};
use waft_widgets::*;
use waft_winui::*;

/// The row height every test row has, so that the list's geometry is known.
const ROW: f64 = 40.0;

/// What a test reads back from its list.
#[derive(Default)]
struct Probe {
    selection: Cell<Option<Handle<ItemsViewSelection>>>,
    invoked: RefCell<Vec<usize>>,
    selection_changes: Cell<usize>,
    /// Indices the template was asked to build.
    built: RefCell<Vec<usize>>,
}

impl Probe {
    fn selected(&self, f: &Fixture) -> Vec<usize> {
        let app = f.cell.borrow();
        self.selection.get().unwrap().selected_indices(&app)
    }

    fn current(&self, f: &Fixture) -> Option<usize> {
        let app = f.cell.borrow();
        self.selection.get().unwrap().current_item_index(&app)
    }
}

/// How the list under test is set up.
#[derive(Clone, Copy)]
struct Setup {
    count: usize,
    mode: ItemsViewSelectionMode,
    invoked: bool,
    /// A row the user cannot select.
    unselectable: Option<usize>,
    /// A disabled row.
    disabled: Option<usize>,
    /// The rows' extent, told to the view.
    item_extent: Option<f64>,
}

impl Setup {
    fn new(count: usize, mode: ItemsViewSelectionMode) -> Self {
        Setup {
            count,
            mode,
            invoked: false,
            unselectable: None,
            disabled: None,
            item_extent: None,
        }
    }
}

fn fixture(setup: Setup) -> (Fixture, Rc<Probe>) {
    let probe = Rc::new(Probe::default());
    let shared = probe.clone();
    let f = Fixture::with_root([400, 400], move |app| {
        winui_gallery::install_fonts(app);
        let selection = ItemsViewSelection::new(app);
        shared.selection.set(Some(selection));
        let probe = shared.clone();
        let mut view = ItemsView::new(setup.count, {
            let probe = probe.clone();
            move |_, _, index| {
                probe.built.borrow_mut().push(index);
                let mut container = ItemContainer::new(
                    SizedBox::new().height(ROW).child(
                        Padding::new(EdgeInsetsGeometry::from_ltrb(12.0, 0.0, 0.0, 0.0))
                            .child(Align::new().child(Text::new(format!("Row {index}")))),
                    ),
                );
                if setup.unselectable == Some(index) {
                    container =
                        container.can_user_select(ItemContainerUserSelectMode::UserCannotSelect);
                }
                if setup.disabled == Some(index) {
                    container = container.is_enabled(false);
                }
                container
            }
        })
        .selection_mode(setup.mode)
        .selection(selection)
        .is_item_invoked_enabled(setup.invoked)
        .item_invoked({
            let probe = probe.clone();
            move |_, index| probe.invoked.borrow_mut().push(index)
        })
        .selection_changed(move |_| {
            probe
                .selection_changes
                .set(probe.selection_changes.get() + 1)
        });
        if let Some(extent) = setup.item_extent {
            view = view.item_extent(extent);
        }
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    ThemeScope::new(
                        Theme::Light,
                        ColoredBox::new(Color::from_argb(255, 255, 255, 255)).child(
                            FocusScope::new(
                                Column::new().children([
                                    // A control before the list, so that Tab has somewhere to come from.
                                    SizedBox::new()
                                        .height(40.0)
                                        .child(Button::text(
                                            "Before",
                                            waft_foundation::Listener::new(|_| {}),
                                        ))
                                        .into_widget(),
                                    SizedBox::new()
                                        .height(240.0)
                                        .child(view.clone())
                                        .into_widget(),
                                ]),
                            )
                            .autofocus(true),
                        ),
                    )
                    .into_widget()
                })
                .into_widget(),
        );
    });
    (f, probe)
}

fn press(f: &mut Fixture, physical: Physical, logical: Key) {
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
    );
}

fn release(f: &mut Fixture, physical: Physical, logical: Key) {
    send_key(f, KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)));
}

fn send_key(f: &mut Fixture, event: KeyEvent) {
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

fn key(f: &mut Fixture, physical: Physical, logical: Key) {
    press(f, physical, logical);
    release(f, physical, logical);
    settle(f);
}

/// `logical` with `modifier` held.
fn chord(f: &mut Fixture, modifier: (Physical, Key), physical: Physical, logical: Key) {
    press(f, modifier.0, modifier.1);
    key(f, physical, logical);
    release(f, modifier.0, modifier.1);
}

const CONTROL: (Physical, Key) = (Physical::CONTROL_LEFT, Key::CONTROL_LEFT);
const SHIFT: (Physical, Key) = (Physical::SHIFT_LEFT, Key::SHIFT_LEFT);

/// A touch tap on a row's text; `after` apart from the previous tap.
fn tap_row(f: &mut Fixture, index: usize) {
    let point = f.find(&format!("Row {index}"));
    f.send(PointerChange::Down, point);
    f.at += Duration::from_millis(20);
    f.send(PointerChange::Up, point);
    f.pump();
}

/// A tap with `modifier` held.
fn modified_tap(f: &mut Fixture, modifier: (Physical, Key), index: usize) {
    press(f, modifier.0, modifier.1);
    tap_row(f, index);
    release(f, modifier.0, modifier.1);
}

fn settle(f: &mut Fixture) {
    for _ in 0..12 {
        f.frame_after(Duration::from_millis(20));
    }
}

/// Whether the row's label has the primary focus.
fn row_focused(f: &Fixture, index: usize) -> bool {
    let point = f.find(&format!("Row {index}"));
    let mut app = f.cell.borrow_mut();
    let Some(focus) = primary_focus(&mut app) else {
        return false;
    };
    let Some(context) = focus.context(&app) else {
        return false;
    };
    let render = context.find_render_object(&app).unwrap().as_box().unwrap();
    let origin = render.local_to_global(&app, Offset::ZERO, None);
    let rect = origin & render.size(&app);
    rect.contains(point)
}

/// The pixel at `point`.
fn pixel(f: &Fixture, point: Offset) -> [u8; 3] {
    let pixels = f.view.pixels.borrow();
    let width = f.view.size[0] as usize;
    let index = (point.dy() as usize * width + point.dx() as usize) * 4;
    [pixels[index], pixels[index + 1], pixels[index + 2]]
}

/// A point in the row's empty area, right of its label.
fn row_ground(f: &Fixture, index: usize) -> Offset {
    let label = f.find(&format!("Row {index}"));
    Offset::new(300.0, label.dy())
}

/// The row's left edge, inside the selection visual's 3 pixel border.
fn row_edge(f: &Fixture, index: usize) -> Offset {
    let label = f.find(&format!("Row {index}"));
    Offset::new(1.0, label.dy())
}

fn is_accent(rgb: [u8; 3]) -> bool {
    let [r, g, b] = rgb;
    b > 150 && r < 60 && g < 140
}

// MARK: Selection modes

#[test]
fn single_click_selects_one_and_control_click_deselects() {
    let (mut f, p) = fixture(Setup::new(5, ItemsViewSelectionMode::Single));
    tap_row(&mut f, 1);
    tap_row(&mut f, 3);
    assert_eq!(p.selected(&f), [3]);
    modified_tap(&mut f, CONTROL, 3);
    assert_eq!(p.selected(&f), Vec::<usize>::new());
    assert_eq!(p.selection_changes.get(), 3);
}

#[test]
fn multiple_click_toggles_and_shift_follows_the_anchor() {
    let (mut f, p) = fixture(Setup::new(6, ItemsViewSelectionMode::Multiple));
    tap_row(&mut f, 1);
    tap_row(&mut f, 3);
    assert_eq!(p.selected(&f), [1, 3]);
    modified_tap(&mut f, SHIFT, 5);
    assert_eq!(p.selected(&f), [1, 3, 4, 5]);
    tap_row(&mut f, 1);
    assert_eq!(p.selected(&f), [3, 4, 5]);
}

#[test]
fn extended_click_control_and_shift() {
    let (mut f, p) = fixture(Setup::new(6, ItemsViewSelectionMode::Extended));
    tap_row(&mut f, 1);
    modified_tap(&mut f, CONTROL, 4);
    assert_eq!(p.selected(&f), [1, 4]);
    modified_tap(&mut f, SHIFT, 2);
    assert_eq!(
        p.selected(&f),
        [2, 3, 4],
        "the range from the anchor replaces the selection"
    );
    tap_row(&mut f, 0);
    assert_eq!(p.selected(&f), [0]);
}

#[test]
fn none_selects_nothing() {
    let (mut f, p) = fixture(Setup::new(3, ItemsViewSelectionMode::None));
    tap_row(&mut f, 1);
    assert_eq!(p.selected(&f), Vec::<usize>::new());
    // A container that can neither be selected nor invoked tells its view nothing.
    assert_eq!(p.current(&f), None);
}

#[test]
fn control_a_selects_all_in_multiple_and_extended_modes_only() {
    for (mode, all) in [
        (ItemsViewSelectionMode::Single, false),
        (ItemsViewSelectionMode::Multiple, true),
        (ItemsViewSelectionMode::Extended, true),
    ] {
        let (mut f, p) = fixture(Setup::new(4, mode));
        tap_row(&mut f, 0);
        chord(&mut f, CONTROL, Physical::KEY_A, Key::KEY_A);
        let expected: Vec<usize> = if all { (0..4).collect() } else { vec![0] };
        assert_eq!(p.selected(&f), expected, "{mode:?}");
    }
}

#[test]
fn an_item_the_user_cannot_select_stays_unselected() {
    let mut setup = Setup::new(5, ItemsViewSelectionMode::Extended);
    setup.unselectable = Some(2);
    let (mut f, p) = fixture(setup);
    tap_row(&mut f, 0);
    modified_tap(&mut f, SHIFT, 4);
    assert_eq!(p.selected(&f), [0, 1, 3, 4]);
}

#[test]
fn the_application_selects_through_the_selection() {
    let (mut f, p) = fixture(Setup::new(5, ItemsViewSelectionMode::Multiple));
    let selection = p.selection.get().unwrap();
    selection.select_all(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(p.selected(&f), [0, 1, 2, 3, 4]);
    assert!(
        is_accent(pixel(&f, row_edge(&f, 3))),
        "a row selected from code shows it"
    );
    selection.deselect(&mut f.cell.borrow_mut(), 1);
    selection.invert_selection(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(p.selected(&f), [1]);
    assert!(!is_accent(pixel(&f, row_edge(&f, 3))));
}

// MARK: Pointer states

#[test]
fn a_row_lights_under_the_mouse_shows_pressed_and_shows_its_selection() {
    let (mut f, _) = fixture(Setup::new(4, ItemsViewSelectionMode::Single));
    settle(&mut f);
    let ground = row_ground(&f, 1);
    let normal = pixel(&f, ground);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    f.send_mouse(PointerChange::Hover, ground, 0);
    f.pump();
    let over = pixel(&f, ground);
    assert!(
        over[0] < normal[0],
        "pointer over darkens the row: {normal:?} then {over:?}"
    );
    f.send_mouse(PointerChange::Down, ground, 1);
    // The tap recognizer shows a press once it is more than a touch (`kPressTimeout`).
    f.frame_after(Duration::from_millis(150));
    // `SubtleFillColorTertiary`, pressed, is fainter than `SubtleFillColorSecondary`, over.
    let pressed = pixel(&f, ground);
    assert!(
        over[0] < pressed[0] && pressed[0] < normal[0],
        "pressed tints the row less than over: {over:?} then {pressed:?}"
    );
    assert!(!is_accent(pixel(&f, row_edge(&f, 1))));
    f.send_mouse(PointerChange::Up, ground, 0);
    settle(&mut f);
    assert!(
        is_accent(pixel(&f, row_edge(&f, 1))),
        "the selection visual's accent border"
    );
    f.capture("items-view");
}

#[test]
fn multiple_mode_shows_check_boxes() {
    let (mut f, _) = fixture(Setup::new(4, ItemsViewSelectionMode::Multiple));
    tap_row(&mut f, 1);
    settle(&mut f);
    // `ItemContainerCheckboxMargin` places the 20 pixel box 4 pixels in from the right and 2
    // above the top: a point in its lower left, clear of the check glyph and the selection
    // visual's border.
    let label = f.find("Row 1");
    let in_box = Offset::new(400.0 - 4.0 - 17.0, label.dy() - ROW / 2.0 + 15.0);
    assert!(
        is_accent(pixel(&f, in_box)),
        "the selected row's box is checked"
    );
    let unselected = Offset::new(in_box.dx(), in_box.dy() + ROW);
    assert!(!is_accent(pixel(&f, unselected)));
}

// MARK: Keyboard and focus

#[test]
fn tab_enters_at_the_first_row_and_selection_follows_the_arrow_keys() {
    let (mut f, p) = fixture(Setup::new(5, ItemsViewSelectionMode::Single));
    settle(&mut f);
    key(&mut f, Physical::TAB, Key::TAB);
    key(&mut f, Physical::TAB, Key::TAB);
    assert!(row_focused(&f, 0));
    assert_eq!(
        p.selected(&f),
        Vec::<usize>::new(),
        "focus moving in selects nothing"
    );
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert!(row_focused(&f, 2));
    assert_eq!(p.selected(&f), [2]);
    chord(&mut f, CONTROL, Physical::ARROW_UP, Key::ARROW_UP);
    assert!(row_focused(&f, 1));
    assert_eq!(p.selected(&f), [2], "Control moves the focus alone");
    key(&mut f, Physical::SPACE, Key::SPACE);
    assert_eq!(p.selected(&f), [1], "Space selects the focused row");
}

#[test]
fn the_arrow_keys_pass_a_disabled_row() {
    let mut setup = Setup::new(4, ItemsViewSelectionMode::Single);
    setup.disabled = Some(1);
    let (mut f, p) = fixture(setup);
    tap_row(&mut f, 0);
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(p.current(&f), Some(2));
}

#[test]
fn shift_with_the_arrow_keys_extends_in_extended_and_multiple_modes() {
    let (mut f, p) = fixture(Setup::new(6, ItemsViewSelectionMode::Extended));
    tap_row(&mut f, 1);
    chord(&mut f, SHIFT, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    chord(&mut f, SHIFT, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(p.selected(&f), [1, 2, 3]);
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(
        p.selected(&f),
        [4],
        "a plain arrow selects the focused row alone"
    );

    let (mut f, p) = fixture(Setup::new(6, ItemsViewSelectionMode::Multiple));
    tap_row(&mut f, 1);
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(p.selected(&f), [1], "a plain arrow moves the focus alone");
    chord(&mut f, SHIFT, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert_eq!(p.selected(&f), [1, 2, 3]);
}

// MARK: Invoking

#[test]
fn with_a_selection_mode_a_double_tap_or_enter_invokes() {
    let mut setup = Setup::new(4, ItemsViewSelectionMode::Single);
    setup.invoked = true;
    let (mut f, p) = fixture(setup);
    tap_row(&mut f, 2);
    assert!(p.invoked.borrow().is_empty(), "a tap selects");
    f.at += Duration::from_millis(100);
    tap_row(&mut f, 2);
    assert_eq!(
        *p.invoked.borrow(),
        [2],
        "a second tap soon after is a double tap"
    );
    key(&mut f, Physical::ENTER, Key::ENTER);
    assert_eq!(*p.invoked.borrow(), [2, 2]);
    key(&mut f, Physical::SPACE, Key::SPACE);
    assert_eq!(
        *p.invoked.borrow(),
        [2, 2],
        "Space selects rather than invokes"
    );
}

#[test]
fn without_a_selection_mode_a_tap_invokes() {
    let mut setup = Setup::new(4, ItemsViewSelectionMode::None);
    setup.invoked = true;
    let (mut f, p) = fixture(setup);
    tap_row(&mut f, 1);
    assert_eq!(*p.invoked.borrow(), [1]);
}

#[test]
fn invoking_is_off_unless_enabled() {
    let (mut f, p) = fixture(Setup::new(4, ItemsViewSelectionMode::None));
    tap_row(&mut f, 1);
    key(&mut f, Physical::ENTER, Key::ENTER);
    assert!(p.invoked.borrow().is_empty());
}

// MARK: A long list

#[test]
fn a_long_list_builds_only_what_is_near_the_view_and_end_reaches_the_last_row() {
    let mut setup = Setup::new(10_000, ItemsViewSelectionMode::Single);
    setup.item_extent = Some(ROW);
    let (mut f, p) = fixture(setup);
    settle(&mut f);
    let built = p.built.borrow().iter().copied().max().unwrap();
    assert!(
        built < 30,
        "only the rows in and near the view are built, up to {built}"
    );
    tap_row(&mut f, 0);
    key(&mut f, Physical::END, Key::END);
    settle(&mut f);
    assert!(row_focused(&f, 9_999));
    assert_eq!(p.selected(&f), [9_999]);
    let mut distinct = p.built.borrow().clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert!(
        distinct.len() < 100,
        "End skips the rows between, building {}",
        distinct.len()
    );
    key(&mut f, Physical::HOME, Key::HOME);
    settle(&mut f);
    assert!(row_focused(&f, 0));
}

#[test]
fn without_an_item_extent_end_still_reaches_the_last_row() {
    let (mut f, p) = fixture(Setup::new(300, ItemsViewSelectionMode::Single));
    tap_row(&mut f, 0);
    key(&mut f, Physical::END, Key::END);
    settle(&mut f);
    assert!(row_focused(&f, 299));
    assert_eq!(p.selected(&f), [299]);
}

#[test]
fn page_down_moves_to_the_last_row_in_view_then_a_page_further() {
    let (mut f, p) = fixture(Setup::new(100, ItemsViewSelectionMode::Single));
    tap_row(&mut f, 0);
    // 240 pixels show six rows of 40: the last wholly in view is row 5.
    key(&mut f, Physical::PAGE_DOWN, Key::PAGE_DOWN);
    assert_eq!(p.current(&f), Some(5));
    key(&mut f, Physical::PAGE_DOWN, Key::PAGE_DOWN);
    settle(&mut f);
    assert_eq!(p.current(&f), Some(11));
    assert!(row_focused(&f, 11));
    key(&mut f, Physical::PAGE_UP, Key::PAGE_UP);
    settle(&mut f);
    assert_eq!(
        p.current(&f),
        Some(6),
        "the first row wholly in view going back"
    );
}

#[test]
fn the_gallery_page_selects_invokes_and_shows_both_themes() {
    use crate::common::GalleryFixtureExt;
    let mut f = Fixture::for_feature([1080, 1400], winui_gallery::Feature::ItemsView);
    f.tap("Science daily");
    f.find("Selected: Science daily");
    f.capture("gallery_items_view_light");
    f.tap("Dark theme");
    f.capture("gallery_items_view_dark");
}

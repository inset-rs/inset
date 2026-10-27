//! TabView collection changes exercise real layout, input, focus and retained pages.
#![feature(arbitrary_self_types)]
mod common;

use inset_embedder::{
    Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, Rect,
};
use inset_foundation::{App, Handle, Listener, ValueKey};
use inset_gestures::GestureBinding;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::time::Duration;
use std::{cell::Cell, rc::Rc};

/// Gives tests access to the owner without replacing the TabView.
#[derive(Clone, Debug, Default)]
struct Probe(Rc<Cell<Option<Handle<PageState>>>>);

#[derive(Debug)]
struct Page(Probe, usize);

struct PageState {
    state: StateData<Page>,
    items: Vec<usize>,
    selected: Option<usize>,
    disabled: Vec<usize>,
    hidden: Vec<usize>,
    theme: Theme,
    mode: TabViewWidthMode,
    footer: bool,
    reorder: bool,
    drag: bool,
    allow_drop: bool,
    cancel: bool,
    events: Vec<String>,
    custom_header: Option<Rc<GlobalKey>>,
    custom_feedback: bool,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            items: (0..self.1).collect(),
            selected: Some(0),
            disabled: Vec::new(),
            hidden: Vec::new(),
            theme: Theme::Light,
            mode: TabViewWidthMode::Equal,
            footer: false,
            reorder: true,
            drag: false,
            allow_drop: true,
            cancel: false,
            events: Vec::new(),
            custom_header: None,
            custom_feedback: false,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.0.set(Some(self));
        let state = app.get(self);
        let tabs = state
            .items
            .iter()
            .map(|id| {
                if *id == 0
                    && let Some(key) = &state.custom_header
                {
                    return TabViewItem::new(
                        id.to_string(),
                        SizedBox::new()
                            .key(key.clone())
                            .child(Text::new("Custom tab")),
                        Center::new().child(Counter(*id)),
                    );
                }
                TabViewItem::text(
                    id.to_string(),
                    format!("Tab {id}"),
                    Center::new().child(Counter(*id)),
                )
                .is_enabled(!state.disabled.contains(id))
                .is_visible(!state.hidden.contains(id))
            })
            .collect();
        let mut tabs = TabView::new(tabs, state.selected, move |app, index| {
            self.set_state(app, |s| s.selected = index);
        })
        .can_reorder_tabs(state.reorder)
        .can_drag_tabs(state.drag)
        .allow_drop_tabs(state.allow_drop)
        .tab_reorder_requested(move |app, args| {
            self.set_state(app, |state| {
                let item = state.items.remove(args.old_index);
                state.items.insert(args.new_index, item);
                state.selected = args.selected_index;
                state
                    .events
                    .push(format!("reorder {} {}", args.old_index, args.new_index));
            });
        })
        .tab_drag_starting(move |app, args| {
            args.cancel = app.get(self).cancel;
            app.get_mut(self).events.push("start".into());
        })
        .tab_drag_completed(move |app, args| {
            app.get_mut(self)
                .events
                .push(format!("complete {}", args.was_accepted))
        })
        .tab_dropped_outside(move |app, _| app.get_mut(self).events.push("outside".into()))
        .tab_width_mode(state.mode)
        .tab_close_requested(Rc::new(move |app, args| {
            self.set_state(app, |s| {
                s.items.remove(args.index);
            });
        }));
        if state.custom_feedback {
            tabs = tabs.tab_drag_feedback_builder(|_, _, _| {
                ColoredBox::new(inset_embedder::Color::from_argb(255, 255, 0, 255))
                    .child(Text::new("Independent preview"))
                    .into_widget()
            });
        }
        if state.footer {
            tabs = tabs.tab_strip_footer(Text::new("Footer"));
        }
        ThemeScope::new(state.theme, tabs).into_widget()
    }
}

#[derive(Debug)]
struct Counter(usize);

struct CounterState {
    state: StateData<Counter>,
    count: usize,
}

impl StatefulWidget for Counter {
    type State = CounterState;

    fn create_state(&self) -> CounterState {
        CounterState {
            state: StateData::new(),
            count: 0,
        }
    }
}

impl State for CounterState {
    type Widget = Counter;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        Button::text(
            format!("Page {} count {}", self.widget(app).0, app.get(self).count),
            Listener::new(move |app| self.set_state(app, |s| s.count += 1)),
        )
        .into_widget()
    }
}

fn fixture(count: usize) -> (Fixture, Probe) {
    let p = Probe::default();
    let page = p.clone();
    let f = Fixture::with_root([480, 220], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(page.clone(), count).into_widget()),
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
    (f, p)
}

fn settle(f: &mut Fixture) {
    for _ in 0..3 {
        f.pump();
    }
}

fn click(f: &mut Fixture, point: Offset) {
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    settle(f);
}

#[test]
fn selection_retains_page_state_and_close_reconciles_collection() {
    let (mut f, p) = fixture(3);
    settle(&mut f);
    let page = f.find("Page 0 count 0");
    click(&mut f, page);
    let second = f.find("Tab 1");
    click(&mut f, second);
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).selected, Some(1));
    let first = f.find("Tab 0");
    click(&mut f, first);
    f.find("Page 0 count 1");
    f.capture("tab_view_light");
    p.0.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.theme = Theme::Dark);
    settle(&mut f);
    f.capture("tab_view_dark");
    // Equal tabs use (480 - header 2 - add 36 - edge padding 8) / 3.
    click(&mut f, Offset::new(6.0 + 434.0 / 3.0 - 20.0, 24.0));
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).items, vec![1, 2]);
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).selected, Some(0));
    f.find("Page 1 count 0");
}

#[test]
fn offscreen_selection_realizes_only_a_window_and_footer_stays_measured() {
    let (mut f, p) = fixture(1000);
    settle(&mut f);
    assert!(f.elements().len() < 2000, "headers must remain virtualized");
    p.0.get().unwrap().set_state(&mut f.cell.borrow_mut(), |s| {
        s.selected = Some(999);
        s.footer = true;
    });
    settle(&mut f);
    let last = f.find("Tab 999");
    assert!(
        last.dx() > 0.0 && last.dx() < 480.0,
        "selected tab was not revealed: {last:?}"
    );
    f.find("Footer");
    f.capture("tab_view_overflow");
}

#[test]
fn compact_and_content_width_modes_layout() {
    let (mut f, p) = fixture(3);
    settle(&mut f);
    for mode in [TabViewWidthMode::Compact, TabViewWidthMode::SizeToContent] {
        p.0.get()
            .unwrap()
            .set_state(&mut f.cell.borrow_mut(), |s| s.mode = mode);
        settle(&mut f);
        f.capture(if mode == TabViewWidthMode::Compact {
            "tab_view_compact"
        } else {
            "tab_view_content"
        });
    }
}

fn move_tab_pointer(f: &mut Fixture, from: Offset, to: Offset) {
    let mut app = f.cell.borrow_mut();
    GestureBinding::instance(&mut app).handle_pointer_data_packet(
        &mut app,
        PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Move,
            kind: PointerDeviceKind::Mouse,
            time_stamp: f.at,
            pointer_identifier: f.mouse_pointer,
            physical_x: to.dx(),
            physical_y: to.dy(),
            physical_delta_x: to.dx() - from.dx(),
            physical_delta_y: to.dy() - from.dy(),
            buttons: 1,
            ..Default::default()
        }]),
    );
    drop(app);
    f.cell.checkpoint();
    f.pump();
}
fn pointer_reorder(f: &mut Fixture, from: Offset, to: Offset) {
    f.send_mouse(PointerChange::Down, from, 1);
    move_tab_pointer(f, from, to);
    f.send_mouse(PointerChange::Up, to, 0);
    settle(f);
}
fn key(f: &mut Fixture, down: bool, physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) {
    let event = if down {
        KeyEvent::Down(KeyDownEvent::new(physical, logical, Duration::ZERO))
    } else {
        KeyEvent::Up(KeyUpEvent::new(physical, logical, Duration::ZERO))
    };
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
    settle(f);
}
#[test]
fn pointer_reorder_preserves_selection_page_and_emits_no_external_events_by_default() {
    let (mut f, p) = fixture(3);
    settle(&mut f);
    let page = f.find("Page 0 count 0");
    click(&mut f, page);
    let start = f.find("Tab 0");
    let end = Offset::new(419.0, start.dy());
    pointer_reorder(&mut f, start, end);
    let app = f.cell.borrow();
    let state = app.get(p.0.get().unwrap());
    assert_eq!(state.items, vec![1, 2, 0]);
    assert_eq!(state.selected, Some(2));
    assert_eq!(state.events, vec!["reorder 0 2"]);
    drop(app);
    f.find("Page 0 count 1");
    f.capture("tab_view_reordered");
}
#[test]
fn keyboard_reorder_is_scoped_to_header_and_preserves_focused_identity() {
    let (mut f, p) = fixture(3);
    settle(&mut f);
    f.focus("Tab 0");
    key(
        &mut f,
        true,
        PhysicalKeyboardKey::ALT_LEFT,
        LogicalKeyboardKey::ALT_LEFT,
    );
    key(
        &mut f,
        true,
        PhysicalKeyboardKey::SHIFT_LEFT,
        LogicalKeyboardKey::SHIFT_LEFT,
    );
    key(
        &mut f,
        true,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    key(
        &mut f,
        false,
        PhysicalKeyboardKey::ARROW_RIGHT,
        LogicalKeyboardKey::ARROW_RIGHT,
    );
    key(
        &mut f,
        false,
        PhysicalKeyboardKey::SHIFT_LEFT,
        LogicalKeyboardKey::SHIFT_LEFT,
    );
    key(
        &mut f,
        false,
        PhysicalKeyboardKey::ALT_LEFT,
        LogicalKeyboardKey::ALT_LEFT,
    );
    let app = f.cell.borrow();
    let state = app.get(p.0.get().unwrap());
    assert_eq!(state.items, vec![1, 0, 2]);
    assert_eq!(state.selected, Some(1));
}
#[test]
fn drag_start_can_cancel_and_disabled_drop_does_not_reorder() {
    let (mut f, p) = fixture(3);
    settle(&mut f);
    let owner = p.0.get().unwrap();
    owner.set_state(&mut f.cell.borrow_mut(), |state| {
        state.drag = true;
        state.cancel = true;
    });
    settle(&mut f);
    let start = f.find("Tab 0");
    pointer_reorder(&mut f, start, Offset::new(419.0, start.dy()));
    assert_eq!(f.cell.borrow().get(owner).items, vec![0, 1, 2]);
    assert_eq!(f.cell.borrow().get(owner).events, vec!["start"]);
    owner.set_state(&mut f.cell.borrow_mut(), |state| {
        state.cancel = false;
        state.allow_drop = false;
        state.events.clear();
    });
    settle(&mut f);
    let start = f.find("Tab 0");
    pointer_reorder(&mut f, start, Offset::new(419.0, start.dy()));
    assert_eq!(f.cell.borrow().get(owner).items, vec![0, 1, 2]);
    assert_eq!(
        f.cell.borrow().get(owner).events,
        vec!["start", "complete false", "outside"]
    );
}

#[derive(Clone, Debug, Default)]
struct CrossProbe(Rc<Cell<Option<Handle<CrossState>>>>);
#[derive(Debug)]
struct CrossPage(CrossProbe);
struct CrossState {
    state: StateData<CrossPage>,
    left: Vec<usize>,
    right: Vec<usize>,
    completed: Option<bool>,
}
impl StatefulWidget for CrossPage {
    type State = CrossState;
    fn create_state(&self) -> Self::State {
        CrossState {
            state: StateData::new(),
            left: vec![0, 1],
            right: vec![2],
            completed: None,
        }
    }
}
impl State for CrossState {
    type Widget = CrossPage;
    inset_widgets::state_accessors!();
    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.0.set(Some(self));
        let items = |ids: &[usize]| {
            ids.iter()
                .map(|id| {
                    TabViewItem::text(
                        id.to_string(),
                        format!("Cross {id}"),
                        Text::new(format!("Content {id}")),
                    )
                })
                .collect()
        };
        let left = TabView::new(items(&app.get(self).left), Some(0), |_, _| {})
            .can_drag_tabs(true)
            .tab_drag_completed(move |app, args| {
                app.get_mut(self).completed = Some(args.was_accepted)
            });
        let right = TabView::new(items(&app.get(self).right), Some(0), |_, _| {})
            .tab_strip_drag_over(|_, args| args.accepted = true)
            .tab_strip_drop(move |app, args| {
                let id = args.item.id.parse::<usize>().unwrap();
                self.set_state(app, |state| {
                    state.left.retain(|item| *item != id);
                    state.right.insert(args.insertion_index, id);
                });
            });
        ThemeScope::new(
            Theme::Light,
            Column::new().children([
                SizedBox::new().height(100.0).child(left).into_widget(),
                SizedBox::new().height(100.0).child(right).into_widget(),
            ]),
        )
        .into_widget()
    }
}
#[test]
fn cross_strip_drop_is_owner_managed_and_completes_source_drag() {
    let probe = CrossProbe::default();
    let copy = probe.clone();
    let mut f = Fixture::with_root([480, 220], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| CrossPage(copy.clone()).into_widget()),
            false,
            true,
            false,
        );
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| Overlay::new().initial_entries([entry]).into_widget())
                .into_widget(),
        );
    });
    settle(&mut f);
    let from = f.find("Cross 0");
    let destination = f.find("Cross 2");
    pointer_reorder(
        &mut f,
        from,
        Offset::new(destination.dx() + 140.0, destination.dy()),
    );
    let app = f.cell.borrow();
    let state = app.get(probe.0.get().unwrap());
    assert_eq!(state.left, vec![1]);
    assert_eq!(state.right, vec![2, 0]);
    assert_eq!(state.completed, Some(true));
}

#[test]
fn custom_keyed_header_drag_keeps_live_subtree_and_accepts_independent_feedback() {
    let (mut f, p) = fixture(3);
    settle(&mut f);
    let owner = p.0.get().unwrap();
    let header_key = Rc::new(GlobalKey::new());
    owner.set_state(&mut f.cell.borrow_mut(), |state| {
        state.custom_header = Some(header_key.clone())
    });
    settle(&mut f);
    let initial = header_key
        .current_context(&mut f.cell.borrow_mut())
        .unwrap();
    let start = f.find("Custom tab");
    pointer_reorder(&mut f, start, Offset::new(419.0, start.dy()));
    assert_eq!(f.cell.borrow().get(owner).items, vec![1, 2, 0]);
    assert_eq!(
        header_key.current_context(&mut f.cell.borrow_mut()),
        Some(initial)
    );
    owner.set_state(&mut f.cell.borrow_mut(), |state| {
        state.custom_feedback = true
    });
    settle(&mut f);
    let start = f.find("Custom tab");
    let end = Offset::new(60.0, start.dy());
    f.send_mouse(PointerChange::Down, start, 1);
    move_tab_pointer(&mut f, start, end);
    f.find("Independent preview");
    let pixels = f.view.pixels.borrow();
    let sample = ((36 * f.view.size[0] + 48) * 4) as usize;
    assert_eq!(
        &pixels[sample..sample + 4],
        &[255, 0, 255, 255],
        "feedback must paint above the original strip"
    );
    drop(pixels);
    f.capture("tab_view_drag_custom_feedback");
    f.send_mouse(PointerChange::Up, end, 0);
    settle(&mut f);
    assert_eq!(f.cell.borrow().get(owner).items, vec![0, 1, 2]);
    assert_eq!(
        header_key.current_context(&mut f.cell.borrow_mut()),
        Some(initial)
    );
}

#[test]
fn held_edge_drag_scrolls_to_unrealized_tabs() {
    let (mut f, p) = fixture(30);
    settle(&mut f);
    let start = f.find("Tab 0");
    let edge = Offset::new(440.0, start.dy());
    f.send_mouse(PointerChange::Down, start, 1);
    move_tab_pointer(&mut f, start, edge);
    f.cell.elapse(Duration::from_millis(250));
    f.pump();
    f.capture("tab_view_drag_live_reorder");
    // Holding the pointer at the viewport edge keeps native auto-scrolling alive.
    for _ in 0..15 {
        f.pump();
    }
    f.send_mouse(PointerChange::Up, edge, 0);
    settle(&mut f);
    let app = f.cell.borrow();
    let state = app.get(p.0.get().unwrap());
    let moved_to = state.items.iter().position(|id| *id == 0).unwrap();
    assert!(
        moved_to > 10,
        "edge drag must reach beyond initially realized headers: {moved_to}"
    );
    assert_eq!(state.selected, Some(moved_to));
}

fn control_key(
    f: &mut Fixture,
    physical: PhysicalKeyboardKey,
    logical: LogicalKeyboardKey,
    shift: bool,
) {
    key(
        f,
        true,
        PhysicalKeyboardKey::CONTROL_LEFT,
        LogicalKeyboardKey::CONTROL_LEFT,
    );
    if shift {
        key(
            f,
            true,
            PhysicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::SHIFT_LEFT,
        );
    }
    key(f, true, physical, logical);
    key(f, false, physical, logical);
    if shift {
        key(
            f,
            false,
            PhysicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::SHIFT_LEFT,
        );
    }
    key(
        f,
        false,
        PhysicalKeyboardKey::CONTROL_LEFT,
        LogicalKeyboardKey::CONTROL_LEFT,
    );
}

#[test]
fn control_tab_moves_page_focus_and_close_recovers_adjacent_header_focus() {
    let (mut f, p) = fixture(3);
    settle(&mut f);
    let page = f.find("Page 0 count 0");
    click(&mut f, page);
    f.focus("Page 0 count 1");
    control_key(
        &mut f,
        PhysicalKeyboardKey::TAB,
        LogicalKeyboardKey::TAB,
        false,
    );
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).selected, Some(1));
    key(
        &mut f,
        true,
        PhysicalKeyboardKey::SPACE,
        LogicalKeyboardKey::SPACE,
    );
    key(
        &mut f,
        false,
        PhysicalKeyboardKey::SPACE,
        LogicalKeyboardKey::SPACE,
    );
    f.find("Page 1 count 1");
    control_key(
        &mut f,
        PhysicalKeyboardKey::TAB,
        LogicalKeyboardKey::TAB,
        true,
    );
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).selected, Some(0));
    key(
        &mut f,
        true,
        PhysicalKeyboardKey::SPACE,
        LogicalKeyboardKey::SPACE,
    );
    key(
        &mut f,
        false,
        PhysicalKeyboardKey::SPACE,
        LogicalKeyboardKey::SPACE,
    );
    f.find("Page 0 count 2");
    let tab = f.find("Tab 1");
    click(&mut f, tab);
    f.focus("Tab 1");
    let adjacent_focus = primary_focus(&mut f.cell.borrow_mut());
    let tab = f.find("Tab 2");
    click(&mut f, tab);
    f.focus("Tab 2");
    control_key(
        &mut f,
        PhysicalKeyboardKey::F4,
        LogicalKeyboardKey::F4,
        false,
    );
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).items, vec![0, 1]);
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).selected, Some(1));
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), adjacent_focus);
    // Closing the last selected page while its content owns focus requires the deferred selection update.
    let page = f.find("Page 1 count 1");
    click(&mut f, page);
    f.focus("Page 1 count 2");
    control_key(
        &mut f,
        PhysicalKeyboardKey::F4,
        LogicalKeyboardKey::F4,
        false,
    );
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).selected, Some(0));
    key(
        &mut f,
        true,
        PhysicalKeyboardKey::SPACE,
        LogicalKeyboardKey::SPACE,
    );
    key(
        &mut f,
        false,
        PhysicalKeyboardKey::SPACE,
        LogicalKeyboardKey::SPACE,
    );
    f.find("Page 0 count 3");
}

/// Collection reconciliation must finish before the first layout after removal.
#[test]
fn collection_replacement_is_visible_in_the_first_frame() {
    for (initial_selection, removed, disabled, hidden, expected) in [
        (Some(3), 3, vec![], vec![], 2),
        (Some(3), 3, vec![2], vec![0], 1),
        (Some(3), 0, vec![], vec![], 3),
        (None, 1, vec![], vec![], 2),
        (None, 3, vec![2], vec![0], 1),
    ] {
        let (mut fixture, probe) = fixture(4);
        let owner = probe.0.get().unwrap();
        owner.set_state(&mut fixture.cell.borrow_mut(), |state| {
            state.selected = initial_selection;
            state.disabled = disabled;
            state.hidden = hidden;
        });
        settle(&mut fixture);
        owner.set_state(&mut fixture.cell.borrow_mut(), |state| {
            state.items.remove(removed);
        });
        fixture.frame_after(Duration::from_millis(20));
        // No second build is allowed here: selection_changed only queued that frame.
        fixture.find(&format!("Page {expected} count 0"));
        let state = fixture.cell.borrow();
        let selected = state.get(owner).selected.unwrap();
        assert_eq!(state.get(owner).items[selected], expected);
    }
}

/// The bounds of the laid-out elements keyed `name` that have any width.
fn named_bounds(f: &Fixture, name: &'static str) -> Vec<Rect> {
    let expected = ValueKey::new(name);
    let elements = f.elements();
    let app = f.cell.borrow();
    elements
        .into_iter()
        .filter(|element| {
            element
                .widget(&app)
                .key()
                .is_some_and(|key| key.as_ref().eq_key(&expected))
        })
        .filter_map(|element| {
            let object = element.find_render_object(&app)?.as_box()?;
            let size = object.size(&app);
            (size.width() > 0.0).then(|| object.local_to_global(&app, Offset::ZERO, None) & size)
        })
        .collect()
}

/// The colour of the frame's pixel containing `x`, `y`.
fn pixel(f: &Fixture, x: f64, y: f64) -> [u8; 4] {
    let pixels = f.view.pixels.borrow();
    let index = (y as usize * f.view.size[0] as usize + x as usize) * 4;
    pixels[index..index + 4].try_into().unwrap()
}

/// Whether `line` paints the bottom pixel row of its cell at both ends, against the row above.
fn paints_bottom_row(f: &Fixture, line: Rect) -> bool {
    let bottom = line.bottom - 1.0;
    [line.left, line.right - 1.0]
        .into_iter()
        .all(|x| pixel(f, x, bottom) != pixel(f, x, bottom - 1.0))
}

/// Like the source's one-pixel `Border`s, `LeftBottomBorderLine` and `RightBottomBorderLine` of the
/// strip and of its scroll viewer paint the bottom row of their cells.
#[test]
fn the_bottom_border_lines_paint_the_bottom_row_of_their_cells() {
    let (mut f, _) = fixture(2);
    settle(&mut f);
    let lines: Vec<_> = ["LeftBottomBorderLine", "RightBottomBorderLine"]
        .into_iter()
        .flat_map(|name| named_bounds(&f, name))
        .collect();
    // The strip's two lines and the scroll viewer's left one; its right column is empty without
    // scroll buttons.
    assert_eq!(lines.len(), 3, "{lines:?}");
    for line in lines {
        assert!(paints_bottom_row(&f, line), "{line:?}");
    }
}

/// While a tab is dragged (`SingleBottomBorderLine`), the strip's left line runs under the whole
/// strip and no other line is shown.
#[test]
fn a_dragged_tab_leaves_one_bottom_border_line_across_the_strip() {
    let (mut f, _) = fixture(2);
    settle(&mut f);
    let start = f.find("Tab 0");
    f.send_mouse(PointerChange::Down, start, 1);
    move_tab_pointer(&mut f, start, start + Offset::new(60.0, 0.0));
    settle(&mut f);
    let lines = named_bounds(&f, "LeftBottomBorderLine");
    assert!(named_bounds(&f, "RightBottomBorderLine").is_empty());
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        (lines[0].left, lines[0].right),
        (0.0, f64::from(f.view.size[0]))
    );
    assert!(paints_bottom_row(&f, lines[0]));
    f.send_mouse(PointerChange::Up, start, 0);
}

/// Like a page-level container, a TabView fills the width its parent bounds.
#[test]
fn a_tab_view_fills_the_width_it_is_given() {
    let size = common::size_under_center([400, 300], |_| {
        TabView::new(
            vec![TabViewItem::text("one", "One", Text::new("Page"))],
            Some(0),
            |_, _| {},
        )
        .into_widget()
    });
    assert_eq!(size.width(), 400.0);
}

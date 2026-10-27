//! TreeView: the rows a tree flattens to, the chevron, selection in each mode with Multiple
//! mode's partial parents, ItemInvoked, the keyboard (focus and selection movement, expanding and
//! collapsing, Shift+Alt reordering), and dragging rows between and into others, with the drag
//! count, hover expansion and edge scrolling. A page owns the tree, as an application does, and
//! records what the tree asks of it.
#![feature(arbitrary_self_types)]
mod common;

use common::{Fixture, GalleryFixtureExt};
use inset_embedder::{Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind};
use inset_foundation::{App, Handle};
use inset_gestures::GestureBinding;
use inset_rendering::RenderParagraph;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical,
};
use inset_widgets::*;
use inset_winui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use winui_gallery::Feature;

/// A node of the page's tree.
#[derive(Clone, Debug)]
struct Node {
    id: String,
    children: Vec<Node>,
    expanded: bool,
    unrealized: bool,
}

fn node(id: &str, children: Vec<Node>) -> Node {
    Node {
        id: id.to_owned(),
        children,
        expanded: false,
        unrealized: false,
    }
}

/// WinUI's test tree: `Root`, open, with `Root.0`, `Root.1` (with `Root.1.0` and `Root.1.1`)
/// and `Root.2`, then `Lazy`, whose children are not loaded yet.
fn test_tree() -> Vec<Node> {
    let mut root = node(
        "Root",
        vec![
            node("Root.0", vec![]),
            node(
                "Root.1",
                vec![node("Root.1.0", vec![]), node("Root.1.1", vec![])],
            ),
            node("Root.2", vec![]),
        ],
    );
    root.expanded = true;
    let mut lazy = node("Lazy", vec![]);
    lazy.unrealized = true;
    vec![root, lazy]
}

/// Three hundred root nodes.
fn long_tree() -> Vec<Node> {
    (0..300)
        .map(|index| node(&format!("Item {index}"), vec![]))
        .collect()
}

fn nodes(nodes: &[Node]) -> Vec<TreeViewNode> {
    nodes
        .iter()
        .map(|node| {
            TreeViewNode::text(node.id.clone(), node.id.clone())
                .is_expanded(node.expanded)
                .has_unrealized_children(node.unrealized)
                .children(self::nodes(&node.children))
        })
        .collect()
}

fn find<'a>(nodes: &'a mut [Node], id: &str) -> Option<&'a mut Node> {
    for node in nodes.iter_mut() {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find(&mut node.children, id) {
            return Some(found);
        }
    }
    None
}

fn take(nodes: &mut Vec<Node>, id: &str) -> Option<Node> {
    if let Some(index) = nodes.iter().position(|node| node.id == id) {
        return Some(nodes.remove(index));
    }
    nodes
        .iter_mut()
        .find_map(|node| take(&mut node.children, id))
}

fn apply(nodes: &mut Vec<Node>, node_move: &TreeViewNodeMove) {
    let moved = take(nodes, &node_move.node).unwrap();
    let siblings = match &node_move.new_parent {
        Some(parent) => &mut find(nodes, parent).unwrap().children,
        None => nodes,
    };
    siblings.insert(node_move.index, moved);
}

/// The ids of the tree in depth-first order, with ` > ` before each level of children.
fn order(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(|node| {
            if node.children.is_empty() {
                node.id.clone()
            } else {
                format!("{} > [{}]", node.id, order(&node.children))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Gives tests the page's state.
#[derive(Clone, Debug, Default)]
struct Probe(Rc<Cell<Option<Handle<PageState>>>>);

#[derive(Debug)]
struct Page(Probe, Vec<Node>, TreeViewSelectionMode);

struct PageState {
    state: StateData<Page>,
    nodes: Vec<Node>,
    selected: Vec<String>,
    mode: TreeViewSelectionMode,
    theme: Theme,
    can_drag: bool,
    can_reorder: bool,
    enabled: bool,
    cancel_drag: bool,
    events: Vec<String>,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            nodes: self.1.clone(),
            selected: Vec::new(),
            mode: self.2,
            theme: Theme::Light,
            can_drag: true,
            can_reorder: true,
            enabled: true,
            cancel_drag: false,
            events: Vec::new(),
        }
    }
}

impl PageState {
    fn record(self: Handle<Self>, app: &mut App, event: String) {
        self.set_state(app, |state| state.events.push(event));
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.0.set(Some(self));
        let state = app.get(self);
        let tree = TreeView::new(
            nodes(&state.nodes),
            state.selected.clone(),
            move |app, args| {
                self.set_state(app, |state| {
                    state.selected = args.selected_nodes.clone();
                    state.events.push(format!(
                        "selection +{:?} -{:?}",
                        args.added_items, args.removed_items
                    ));
                })
            },
        )
        .selection_mode(state.mode)
        .can_drag_items(state.can_drag)
        .can_reorder_items(state.can_reorder)
        .is_enabled(state.enabled)
        .expanding(move |app, args| {
            self.set_state(app, |state| {
                find(&mut state.nodes, &args.node).unwrap().expanded = true;
                state.events.push(format!("expanding {}", args.node));
            })
        })
        .collapsed(move |app, args| {
            self.set_state(app, |state| {
                find(&mut state.nodes, &args.node).unwrap().expanded = false;
                state.events.push(format!("collapsed {}", args.node));
            })
        })
        .item_invoked(move |app, args| self.record(app, format!("invoked {}", args.invoked_item)))
        .move_requested(move |app, args| {
            self.set_state(app, |state| {
                for node_move in &args.moves {
                    apply(&mut state.nodes, node_move);
                    state.events.push(format!(
                        "move {} to {} at {}",
                        node_move.node,
                        node_move.new_parent.as_deref().unwrap_or("top"),
                        node_move.index
                    ));
                }
            })
        })
        .drag_items_starting(move |app, args| {
            args.cancel = app.get(self).cancel_drag;
            self.record(app, format!("starting {:?}", args.items));
        })
        .drag_items_completed(move |app, args| {
            self.record(
                app,
                format!(
                    "completed {:?} {:?} {}",
                    args.drop_result,
                    args.items,
                    args.new_parent_item.as_deref().unwrap_or("top")
                ),
            )
        });
        let background = ThemeResources::new(state.theme, AccentPalette::default())
            .solid_background_fill_color_base;
        ThemeScope::new(
            state.theme,
            ColoredBox::new(background).child(
                Align::new()
                    .alignment(inset_painting::Alignment::TOP_LEFT.into())
                    .child(SizedBox::new().width(320.0).height(400.0).child(tree)),
            ),
        )
        .into_widget()
    }
}

fn fixture(tree: Vec<Node>, mode: TreeViewSelectionMode) -> (Fixture, Probe) {
    let probe = Probe::default();
    let page = probe.clone();
    let mut f = Fixture::with_root([360, 420], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(page.clone(), tree.clone(), mode).into_widget()),
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
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    (f, probe)
}

impl Probe {
    fn with<R>(&self, f: &Fixture, read: impl FnOnce(&PageState) -> R) -> R {
        let app = f.cell.borrow();
        read(app.get(self.0.get().unwrap()))
    }

    fn set(&self, f: &mut Fixture, change: impl FnOnce(&mut PageState)) {
        {
            let mut app = f.cell.borrow_mut();
            let handle = self.0.get().unwrap();
            change(app.get_mut(handle));
            handle.set_state(&mut app, |_| {});
        }
        f.cell.checkpoint();
        f.pump();
    }

    /// The events since the last call.
    fn events(&self, f: &Fixture) -> Vec<String> {
        let app = f.cell.borrow();
        let handle = self.0.get().unwrap();
        drop(app);
        let mut app = f.cell.borrow_mut();
        std::mem::take(&mut app.get_mut(handle).events)
    }

    fn selected(&self, f: &Fixture) -> Vec<String> {
        self.with(f, |state| state.selected.clone())
    }

    fn order(&self, f: &Fixture) -> String {
        self.with(f, |state| order(&state.nodes))
    }
}

/// Whether the row `text` is shown.
fn shows(f: &Fixture, text: &str) -> bool {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().any(|element| {
        element.render_object(&app).is_some_and(|object| {
            object
                .downcast::<RenderParagraph>(&app)
                .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == text)
        })
    })
}

/// A row's label, which the row's middle is level with.
fn row(f: &Fixture, text: &str) -> Offset {
    f.find(text)
}

/// The row's chevron, `depth` levels in; Multiple mode puts the check box before it.
fn chevron(f: &Fixture, text: &str, depth: f64, multiple: bool) -> Offset {
    let x = if multiple { 52.0 } else { 24.0 } + 16.0 * depth;
    Offset::new(x, row(f, text).dy())
}

/// The row's check box, `depth` levels in.
fn check_box(f: &Fixture, text: &str, depth: f64) -> Offset {
    Offset::new(30.0 + 16.0 * depth, row(f, text).dy())
}

fn click(f: &mut Fixture, point: Offset) {
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
}

fn send_key(f: &mut Fixture, event: KeyEvent) {
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
}

/// A press and release of `logical` with `modifiers` held.
fn key_with(f: &mut Fixture, modifiers: &[(Key, Physical)], logical: Key, physical: Physical) {
    for (logical, physical) in modifiers {
        send_key(
            f,
            KeyEvent::Down(KeyDownEvent::new(*physical, *logical, f.at)),
        );
    }
    send_key(
        f,
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
    );
    send_key(f, KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)));
    for (logical, physical) in modifiers.iter().rev() {
        send_key(f, KeyEvent::Up(KeyUpEvent::new(*physical, *logical, f.at)));
    }
    f.pump();
}

fn key(f: &mut Fixture, logical: Key, physical: Physical) {
    key_with(f, &[], logical, physical);
}

const CONTROL: (Key, Physical) = (Key::CONTROL_LEFT, Physical::CONTROL_LEFT);
const SHIFT: (Key, Physical) = (Key::SHIFT_LEFT, Physical::SHIFT_LEFT);
const ALT: (Key, Physical) = (Key::ALT_LEFT, Physical::ALT_LEFT);

fn down(f: &mut Fixture) {
    key(f, Key::ARROW_DOWN, Physical::ARROW_DOWN);
}

fn move_pointer(f: &mut Fixture, from: Offset, to: Offset) {
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

/// Presses at `from` and drags to `to` in a few steps, without releasing.
fn drag_to(f: &mut Fixture, from: Offset, to: Offset) {
    f.send_mouse(PointerChange::Hover, from, 0);
    f.send_mouse(PointerChange::Down, from, 1);
    let mut at = from;
    for step in 1..=4 {
        let next = from + (to - from) * (step as f64 / 4.0);
        move_pointer(f, at, next);
        at = next;
    }
}

/// Waits `time` with the pointer where it is.
fn hold(f: &mut Fixture, time: Duration) {
    f.cell.elapse(time);
    f.pump();
}

fn release(f: &mut Fixture, at: Offset) {
    f.send_mouse(PointerChange::Up, at, 0);
    for _ in 0..3 {
        f.pump();
    }
}

/// A row's middle, its top fifth, and its bottom fifth: where a drop goes into it, and the
/// edges where the list has the drag. A row is 32 pixels from its label's middle less 15.
fn middle(f: &Fixture, text: &str) -> Offset {
    Offset::new(160.0, row(f, text).dy())
}

fn top_edge(f: &Fixture, text: &str) -> Offset {
    Offset::new(160.0, row(f, text).dy() - 12.0)
}

fn bottom_edge(f: &Fixture, text: &str) -> Offset {
    Offset::new(160.0, row(f, text).dy() + 12.0)
}

fn single() -> (Fixture, Probe) {
    fixture(test_tree(), TreeViewSelectionMode::Single)
}

fn multiple() -> (Fixture, Probe) {
    fixture(test_tree(), TreeViewSelectionMode::Multiple)
}

#[test]
fn the_rows_are_the_open_nodes_and_rows_are_32_pixels_apart() {
    let (f, _) = single();
    for text in ["Root", "Root.0", "Root.1", "Root.2", "Lazy"] {
        assert!(shows(&f, text), "{text}");
    }
    assert!(!shows(&f, "Root.1.0"));
    assert_eq!(row(&f, "Root.0").dy() - row(&f, "Root").dy(), 32.0);
}

/// The colour drawn at `point`.
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

#[test]
fn single_mode_draws_every_node_the_application_selects() {
    let (mut f, p) = single();
    p.set(&mut f, |state| {
        state.selected = vec!["Root.0".to_owned(), "Root.2".to_owned()];
    });
    let background = |f: &Fixture, text: &str| pixel(f, Offset::new(300.0, row(f, text).dy()));
    let (first, second) = (background(&f, "Root.0"), background(&f, "Root.2"));
    assert_eq!(first, second, "both drawn selected");
    assert_ne!(first, background(&f, "Root.1"), "unlike a row not selected");
}

#[test]
fn the_chevron_toggles_on_press_without_invoking_or_selecting() {
    let (mut f, p) = single();
    let point = chevron(&f, "Root.1", 1.0, false);
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    f.pump();
    // The press alone expands the node.
    assert_eq!(p.events(&f), vec!["expanding Root.1"]);
    assert!(shows(&f, "Root.1.0"));
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
    assert!(p.events(&f).is_empty(), "no ItemInvoked and no selection");
    click(&mut f, point);
    assert_eq!(p.events(&f), vec!["collapsed Root.1"]);
    assert!(!shows(&f, "Root.1.0"));
    // A leaf's chevron is hidden but still takes the press.
    let point = chevron(&f, "Root.0", 1.0, false);
    click(&mut f, point);
    assert_eq!(p.events(&f), vec!["expanding Root.0"]);
    // A node with children not loaded yet expands, and the application loads them.
    let point = chevron(&f, "Lazy", 0.0, false);
    click(&mut f, point);
    assert_eq!(p.events(&f), vec!["expanding Lazy"]);
}

#[test]
fn a_click_invokes_then_selects_and_ctrl_click_toggles_the_selection() {
    let (mut f, p) = single();
    let point = row(&f, "Root.0");
    click(&mut f, point);
    assert_eq!(
        p.events(&f),
        vec!["invoked Root.0", "selection +[\"Root.0\"] -[]"]
    );
    let point = row(&f, "Root.2");
    click(&mut f, point);
    assert_eq!(
        p.events(&f),
        vec!["invoked Root.2", "selection +[\"Root.2\"] -[\"Root.0\"]"]
    );
    let point = row(&f, "Root.2");
    let at = f.at;
    send_key(
        &mut f,
        KeyEvent::Down(KeyDownEvent::new(CONTROL.1, CONTROL.0, at)),
    );
    click(&mut f, point);
    let at = f.at;
    send_key(
        &mut f,
        KeyEvent::Up(KeyUpEvent::new(CONTROL.1, CONTROL.0, at)),
    );
    f.pump();
    assert_eq!(
        p.events(&f),
        vec!["invoked Root.2", "selection +[] -[\"Root.2\"]"]
    );
    assert!(p.selected(&f).is_empty());
}

#[test]
fn the_arrow_keys_move_the_selection_with_the_focus_and_ctrl_moves_only_the_focus() {
    let (mut f, p) = single();
    let point = row(&f, "Root");
    click(&mut f, point);
    p.events(&f);
    down(&mut f);
    assert_eq!(p.selected(&f), vec!["Root.0"]);
    key_with(&mut f, &[CONTROL], Key::ARROW_DOWN, Physical::ARROW_DOWN);
    assert_eq!(p.selected(&f), vec!["Root.0"], "Ctrl moves only the focus");
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(
        p.events(&f),
        vec![
            "selection +[\"Root.0\"] -[\"Root\"]",
            "invoked Root.1",
            "selection +[\"Root.1\"] -[\"Root.0\"]",
        ]
    );
    key(&mut f, Key::END, Physical::END);
    assert_eq!(p.selected(&f), vec!["Lazy"]);
    key(&mut f, Key::HOME, Physical::HOME);
    assert_eq!(p.selected(&f), vec!["Root"]);
    // At the first row, Up moves nowhere.
    p.events(&f);
    key(&mut f, Key::ARROW_UP, Physical::ARROW_UP);
    assert!(p.events(&f).is_empty());
    f.capture("tree_view_keyboard_focus_light");
}

#[test]
fn left_and_right_collapse_expand_and_move_to_the_parent_and_first_child() {
    let (mut f, p) = single();
    let point = row(&f, "Root.1");
    click(&mut f, point);
    p.events(&f);
    key(&mut f, Key::ARROW_RIGHT, Physical::ARROW_RIGHT);
    assert_eq!(p.events(&f), vec!["expanding Root.1"]);
    key(&mut f, Key::ARROW_RIGHT, Physical::ARROW_RIGHT);
    // Right on an open node moves the focus to its first child, without selecting it.
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(
        p.events(&f),
        vec![
            "invoked Root.1.0",
            "selection +[\"Root.1.0\"] -[\"Root.1\"]"
        ]
    );
    key(&mut f, Key::ARROW_LEFT, Physical::ARROW_LEFT);
    key(&mut f, Key::ARROW_LEFT, Physical::ARROW_LEFT);
    // Left moved to the parent, then collapsed it, which hid the selected child.
    assert_eq!(
        p.events(&f),
        vec!["selection +[] -[\"Root.1.0\"]", "collapsed Root.1"]
    );
    // A root node has no parent row to move to.
    key(&mut f, Key::HOME, Physical::HOME);
    key(&mut f, Key::ARROW_LEFT, Physical::ARROW_LEFT);
    assert_eq!(
        p.events(&f),
        vec!["selection +[\"Root\"] -[]", "collapsed Root"]
    );
    p.events(&f);
    key(&mut f, Key::ARROW_LEFT, Physical::ARROW_LEFT);
    assert!(p.events(&f).is_empty());
}

#[test]
fn collapsing_the_parent_of_the_selected_node_deselects_it() {
    let (mut f, p) = single();
    let point = chevron(&f, "Root.1", 1.0, false);
    click(&mut f, point);
    let point = row(&f, "Root.1.1");
    click(&mut f, point);
    p.events(&f);
    let point = chevron(&f, "Root", 0.0, false);
    click(&mut f, point);
    assert_eq!(
        p.events(&f),
        vec!["selection +[] -[\"Root.1.1\"]", "collapsed Root"]
    );
}

#[test]
fn shift_alt_and_an_arrow_reorders_among_siblings() {
    let (mut f, p) = single();
    let point = row(&f, "Root.0");
    click(&mut f, point);
    p.events(&f);
    key_with(&mut f, &[SHIFT, ALT], Key::ARROW_DOWN, Physical::ARROW_DOWN);
    key_with(&mut f, &[SHIFT, ALT], Key::ARROW_DOWN, Physical::ARROW_DOWN);
    // `TreeViewKeyboardReorderTest`: Root.1, Root.2, Root.0; the moved node is deselected.
    assert_eq!(
        p.order(&f),
        "Root > [Root.1 > [Root.1.0, Root.1.1], Root.2, Root.0], Lazy"
    );
    assert_eq!(
        p.events(&f),
        vec![
            "move Root.0 to Root at 1",
            "selection +[] -[\"Root.0\"]",
            "move Root.0 to Root at 2",
        ]
    );
    // The last sibling goes no further.
    key_with(&mut f, &[SHIFT, ALT], Key::ARROW_DOWN, Physical::ARROW_DOWN);
    assert!(p.events(&f).is_empty());
    key_with(&mut f, &[SHIFT, ALT], Key::ARROW_UP, Physical::ARROW_UP);
    assert_eq!(p.events(&f), vec!["move Root.0 to Root at 1"]);
    // Without CanReorderItems, the keys move nothing.
    p.set(&mut f, |state| state.can_reorder = false);
    key_with(&mut f, &[SHIFT, ALT], Key::ARROW_UP, Physical::ARROW_UP);
    assert!(p.events(&f).is_empty());
}

#[test]
fn multiple_selection_selects_subtrees_and_shows_partial_parents() {
    let (mut f, p) = multiple();
    let point = chevron(&f, "Root.1", 1.0, true);
    click(&mut f, point);
    p.events(&f);
    let point = check_box(&f, "Root.1.0", 2.0);
    click(&mut f, point);
    assert_eq!(p.events(&f), vec!["selection +[\"Root.1.0\"] -[]"]);
    f.capture("tree_view_multiple_partial_light");
    // A click on a partly checked box unchecks it, with its subtree.
    let point = check_box(&f, "Root.1", 1.0);
    click(&mut f, point);
    assert_eq!(p.events(&f), vec!["selection +[] -[\"Root.1.0\"]"]);
    // Checking a parent selects its whole subtree.
    click(&mut f, point);
    assert_eq!(p.selected(&f), vec!["Root.1", "Root.1.0", "Root.1.1"]);
    p.events(&f);
    // Clicking a row invokes it without selecting it.
    let point = row(&f, "Root.2");
    click(&mut f, point);
    assert_eq!(p.events(&f), vec!["invoked Root.2"]);
    // Space toggles the focused row's check box; the last child checked checks the parent.
    key(&mut f, Key::SPACE, Physical::SPACE);
    key(&mut f, Key::HOME, Physical::HOME);
    down(&mut f);
    key(&mut f, Key::SPACE, Physical::SPACE);
    assert_eq!(
        p.events(&f),
        vec![
            "selection +[\"Root.2\"] -[]",
            "selection +[\"Root.0\", \"Root\"] -[]",
        ]
    );
    // Space on the checked root unchecks everything under it.
    key(&mut f, Key::HOME, Physical::HOME);
    key(&mut f, Key::SPACE, Physical::SPACE);
    assert!(p.selected(&f).is_empty());
}

#[test]
fn clicking_a_partly_checked_box_unchecks_its_subtree() {
    let (mut f, p) = multiple();
    p.set(&mut f, |state| state.selected = vec!["Root.0".to_owned()]);
    let point = check_box(&f, "Root", 0.0);
    click(&mut f, point);
    assert_eq!(p.events(&f), vec!["selection +[] -[\"Root.0\"]"]);
}

/// The drag of `Root.0` towards `to`, held long enough for the gap to open.
fn drag_root_0(f: &mut Fixture, to: Offset) {
    let from = middle(f, "Root.0");
    drag_to(f, from, to);
    hold(f, Duration::from_millis(250));
}

#[test]
fn a_drop_in_the_gap_after_a_row_moves_the_node_there() {
    let (mut f, p) = single();
    let to = bottom_edge(&f, "Root.1");
    drag_root_0(&mut f, to);
    f.capture("tree_view_drag_gap_light");
    release(&mut f, to);
    // `TreeViewDragAndDropBetweenNodes`: Root.1, Root.0, Root.2.
    assert_eq!(
        p.order(&f),
        "Root > [Root.1 > [Root.1.0, Root.1.1], Root.0, Root.2], Lazy"
    );
    assert_eq!(
        p.events(&f),
        vec![
            "starting [\"Root.0\"]",
            "move Root.0 to Root at 1",
            "completed Move [\"Root.0\"] Root",
        ]
    );
}

#[test]
fn a_drop_on_the_middle_of_a_row_moves_the_node_into_it() {
    let (mut f, p) = single();
    let to = middle(&f, "Root.2");
    drag_root_0(&mut f, to);
    f.capture("tree_view_drag_into_light");
    release(&mut f, to);
    // `TreeViewDragAndDropOnNode`: Root.0 becomes Root.2's last child.
    assert_eq!(
        p.order(&f),
        "Root > [Root.1 > [Root.1.0, Root.1.1], Root.2 > [Root.0]], Lazy"
    );
    assert_eq!(
        p.events(&f)[1..],
        [
            "move Root.0 to Root.2 at 0".to_owned(),
            "completed Move [\"Root.0\"] Root.2".to_owned(),
        ]
    );
}

#[test]
fn a_drop_on_the_dragged_row_or_outside_moves_nothing() {
    let (mut f, p) = single();
    let start = middle(&f, "Root.0");
    let to = start + Offset::new(30.0, 0.0);
    drag_to(&mut f, start, to);
    release(&mut f, to);
    assert_eq!(
        p.events(&f),
        vec!["starting [\"Root.0\"]", "completed None [\"Root.0\"] Root"]
    );
    // Released outside the tree.
    let start = middle(&f, "Root.0");
    let outside = Offset::new(340.0, start.dy());
    drag_to(&mut f, start, outside);
    release(&mut f, outside);
    assert_eq!(
        p.events(&f),
        vec!["starting [\"Root.0\"]", "completed None [\"Root.0\"] Root"]
    );
}

#[test]
fn a_dragged_node_collapses_and_cannot_go_into_its_own_children() {
    let (mut f, p) = single();
    let point = chevron(&f, "Root.1", 1.0, false);
    click(&mut f, point);
    p.events(&f);
    let start = middle(&f, "Root.1");
    let near = start + Offset::new(0.0, 8.0);
    drag_to(&mut f, start, near);
    // The drag collapsed Root.1: its children are no rows to drop on.
    assert!(!shows(&f, "Root.1.0"));
    let to = middle(&f, "Root.2");
    move_pointer(&mut f, near, to);
    release(&mut f, to);
    assert_eq!(
        p.events(&f),
        vec![
            "collapsed Root.1",
            "starting [\"Root.1\"]",
            "move Root.1 to Root.2 at 0",
            "completed Move [\"Root.1\"] Root.2",
        ]
    );
}

#[test]
fn a_canceled_drag_or_one_that_cannot_reorder_moves_nothing() {
    let (mut f, p) = single();
    p.set(&mut f, |state| state.cancel_drag = true);
    let to = middle(&f, "Root.2");
    drag_root_0(&mut f, to);
    release(&mut f, to);
    assert_eq!(p.events(&f), vec!["starting [\"Root.0\"]"]);
    assert_eq!(
        p.order(&f),
        "Root > [Root.0, Root.1 > [Root.1.0, Root.1.1], Root.2], Lazy"
    );
    // `TreeViewDisableItemReorderTest`.
    p.set(&mut f, |state| {
        state.cancel_drag = false;
        state.can_reorder = false;
    });
    let to = middle(&f, "Root.2");
    drag_root_0(&mut f, to);
    release(&mut f, to);
    assert_eq!(
        p.events(&f),
        vec!["starting [\"Root.0\"]", "completed None [\"Root.0\"] Root"]
    );
    // `TreeViewDisableItemDragTest`: without CanDragItems neither event is raised.
    p.set(&mut f, |state| {
        state.can_reorder = true;
        state.can_drag = false;
    });
    let to = middle(&f, "Root.2");
    drag_root_0(&mut f, to);
    release(&mut f, to);
    assert!(p.events(&f).is_empty());
    assert_eq!(
        p.order(&f),
        "Root > [Root.0, Root.1 > [Root.1.0, Root.1.1], Root.2], Lazy"
    );
}

#[test]
fn hovering_a_collapsed_row_for_a_second_expands_it() {
    let (mut f, p) = single();
    let to = middle(&f, "Root.1");
    drag_root_0(&mut f, to);
    assert!(!shows(&f, "Root.1.0"));
    hold(&mut f, Duration::from_millis(800));
    assert!(shows(&f, "Root.1.0"));
    release(&mut f, to);
    assert_eq!(
        p.events(&f),
        vec![
            "starting [\"Root.0\"]",
            "expanding Root.1",
            "move Root.0 to Root.1 at 2",
            "completed Move [\"Root.0\"] Root.1",
        ]
    );
}

#[test]
fn a_drag_of_several_selected_nodes_moves_them_all_with_their_count() {
    let (mut f, p) = multiple();
    let point = check_box(&f, "Root.0", 1.0);
    click(&mut f, point);
    let point = check_box(&f, "Root.1", 1.0);
    click(&mut f, point);
    p.events(&f);
    let to = middle(&f, "Root.2");
    let point = middle(&f, "Root.0");
    drag_to(&mut f, point, to);
    f.capture("tree_view_drag_multiple_light");
    release(&mut f, to);
    // `TreeViewMultiSelectDragAndDropOnNode`: both selected roots go into Root.2.
    assert_eq!(
        p.order(&f),
        "Root > [Root.2 > [Root.0, Root.1 > [Root.1.0, Root.1.1]]], Lazy"
    );
    let events = p.events(&f);
    assert_eq!(
        events[0],
        "starting [\"Root.0\", \"Root.1\", \"Root.1.0\", \"Root.1.1\"]"
    );
    assert_eq!(
        events[1..3],
        [
            "move Root.0 to Root.2 at 0".to_owned(),
            "move Root.1 to Root.2 at 1".to_owned(),
        ]
    );
    // A selected node is no drop target.
    assert_eq!(p.selected(&f).len(), 4);
}

#[test]
fn a_drag_of_selected_nodes_between_rows_keeps_their_order() {
    let (mut f, p) = multiple();
    let point = check_box(&f, "Root.0", 1.0);
    click(&mut f, point);
    let point = check_box(&f, "Root.2", 1.0);
    click(&mut f, point);
    p.events(&f);
    // Root.0 and Root.2 dropped after Lazy, past the last row.
    let start = middle(&f, "Root.0");
    let to = Offset::new(160.0, row(&f, "Lazy").dy() + 40.0);
    drag_to(&mut f, start, to);
    hold(&mut f, Duration::from_millis(250));
    release(&mut f, to);
    assert_eq!(
        p.order(&f),
        "Root > [Root.1 > [Root.1.0, Root.1.1]], Lazy, Root.0, Root.2"
    );
}

#[test]
fn a_long_tree_builds_only_the_rows_in_view_and_scrolls_at_its_edge_while_dragging() {
    let (mut f, p) = fixture(long_tree(), TreeViewSelectionMode::Single);
    assert!(shows(&f, "Item 0"));
    assert!(!shows(&f, "Item 60"), "rows far below are not built");
    // The keyboard scrolls the focused row into view.
    let point = row(&f, "Item 0");
    click(&mut f, point);
    key(&mut f, Key::END, Physical::END);
    assert!(shows(&f, "Item 299"));
    key(&mut f, Key::HOME, Physical::HOME);
    key(&mut f, Key::PAGE_DOWN, Physical::PAGE_DOWN);
    assert_eq!(
        p.selected(&f),
        vec!["Item 11"],
        "the last row fully in view"
    );
    key(&mut f, Key::PAGE_DOWN, Physical::PAGE_DOWN);
    assert_eq!(p.selected(&f), vec!["Item 24"], "a page further");
    key(&mut f, Key::HOME, Physical::HOME);
    // Holding a drag near the bottom edge scrolls the list. Over the middle of a row the row has
    // the drag, so the pointer is on the top fifth of the last row in view.
    let start = middle(&f, "Item 1");
    let edge = Offset::new(160.0, 386.0);
    drag_to(&mut f, start, edge);
    for _ in 0..10 {
        hold(&mut f, Duration::from_millis(100));
    }
    assert!(!shows(&f, "Item 2"), "the list scrolled down");
    release(&mut f, edge);
    // Released on the top fifth of a row that took the last drag over, the node goes into that
    // row, as TreeViewItem's drop handler takes it.
    let moved_to = p.with(&f, |state| {
        state
            .nodes
            .iter()
            .position(|node| node.children.iter().any(|child| child.id == "Item 1"))
            .unwrap()
    });
    assert!(
        moved_to > 12,
        "dropped beyond the rows first in view: {moved_to}"
    );
}

#[test]
fn a_disabled_tree_takes_no_input() {
    let (mut f, p) = single();
    p.set(&mut f, |state| state.enabled = false);
    let point = row(&f, "Root.0");
    click(&mut f, point);
    let point = chevron(&f, "Root.1", 1.0, false);
    click(&mut f, point);
    assert!(p.events(&f).is_empty());
}

#[test]
fn the_tree_in_its_states_in_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let name = if theme == Theme::Light {
            "light"
        } else {
            "dark"
        };
        let (mut f, p) = single();
        p.set(&mut f, |state| state.theme = theme);
        f.capture(&format!("tree_view_collapsed_{name}"));
        let point = chevron(&f, "Root.1", 1.0, false);
        click(&mut f, point);
        let point = row(&f, "Root.1.0");
        click(&mut f, point);
        f.send_mouse(PointerChange::Hover, row(&f, "Root.2"), 0);
        f.pump();
        f.capture(&format!("tree_view_expanded_selected_{name}"));
        let to = bottom_edge(&f, "Root.2");
        let point = middle(&f, "Root.0");
        drag_to(&mut f, point, to);
        hold(&mut f, Duration::from_millis(250));
        f.capture(&format!("tree_view_drag_{name}"));
        release(&mut f, to);

        let (mut f, p) = multiple();
        p.set(&mut f, |state| {
            state.theme = theme;
            state.selected = vec!["Root.1.0".to_owned(), "Root.2".to_owned()];
        });
        let point = chevron(&f, "Root.1", 1.0, true);
        click(&mut f, point);
        f.capture(&format!("tree_view_multiple_{name}"));
        let point = check_box(&f, "Root.0", 1.0);
        click(&mut f, point);
        let to = top_edge(&f, "Lazy");
        let point = middle(&f, "Root.0");
        drag_to(&mut f, point, to);
        f.capture(&format!("tree_view_drag_multiple_{name}"));
        release(&mut f, to);
    }
}

#[test]
fn the_gallery_page_in_both_themes() {
    let mut f = Fixture::for_feature([1080, 1500], Feature::TreeView);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    f.capture("tree_view_gallery_light");
    f.tap("Dark theme");
    f.capture("tree_view_gallery_dark");
}

#[test]
fn tab_enters_the_tree_at_one_row_and_selection_mode_none_selects_nothing() {
    let (mut f, p) = fixture(test_tree(), TreeViewSelectionMode::None);
    // TreeViewList is one tab stop: the first row, while no row had the focus.
    key(&mut f, Key::TAB, Physical::TAB);
    key(&mut f, Key::ENTER, Physical::ENTER);
    assert_eq!(p.events(&f), vec!["invoked Root"]);
    let point = row(&f, "Root.2");
    click(&mut f, point);
    down(&mut f);
    key(&mut f, Key::SPACE, Physical::SPACE);
    assert_eq!(p.events(&f), vec!["invoked Root.2", "invoked Lazy"]);
    assert!(p.selected(&f).is_empty());
}

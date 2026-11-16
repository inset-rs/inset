//! CommandBar: commands moving into the overflow as the bar narrows and back as it widens, `DynamicOverflowOrder`, the label positions, toggle buttons, opening and closing with their events, light dismiss, and the keyboard.

use crate::common::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
use waft_embedder::{Color, Offset, PointerChange, Rect};
use waft_foundation::{App, Handle, Listener};
use waft_rendering::RenderParagraph;
use waft_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical,
};
use waft_widgets::*;
use waft_winui::*;

/// How the bar under test is set up.
#[derive(Clone, Copy, Debug)]
struct Setup {
    width: f64,
    label_position: CommandBarDefaultLabelPosition,
    /// `DynamicOverflowOrder` of each of the six primary commands.
    orders: [i32; 6],
    secondary: bool,
    sticky: bool,
    rtl: bool,
}

impl Default for Setup {
    fn default() -> Self {
        Setup {
            width: 640.0,
            label_position: CommandBarDefaultLabelPosition::Bottom,
            orders: [0; 6],
            secondary: false,
            sticky: false,
            rtl: false,
        }
    }
}

const LABELS: [&str; 6] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot"];

/// What the test and the bar's owner share.
#[derive(Default)]
struct Shared {
    open: Cell<bool>,
    width: Cell<f64>,
    checked: Cell<Option<bool>>,
    three_state: Cell<bool>,
    clicks: RefCell<Vec<&'static str>>,
    events: RefCell<Vec<&'static str>>,
    overflow: RefCell<Vec<CommandBarDynamicOverflowAction>>,
    host: Cell<Option<Handle<HostState>>>,
}

#[derive(Debug)]
struct Host {
    setup: Setup,
}

struct HostState {
    state: StateData<Host>,
    shared: Rc<Shared>,
}

thread_local! {
    static SHARED: RefCell<Option<Rc<Shared>>> = const { RefCell::new(None) };
}

impl StatefulWidget for Host {
    type State = HostState;

    fn create_state(&self) -> HostState {
        HostState {
            state: StateData::new(),
            shared: SHARED.with(|shared| shared.borrow().clone().unwrap()),
        }
    }
}

impl State for HostState {
    type Widget = Host;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get(self).shared.host.set(Some(self));
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let setup = self.widget(app).setup;
        let shared = app.get(self).shared.clone();
        let log = |event: &'static str| {
            let shared = shared.clone();
            Listener::new(move |_| shared.events.borrow_mut().push(event))
        };
        let primary: Vec<CommandBarElement> = LABELS
            .iter()
            .zip(setup.orders)
            .map(|(&label, order)| {
                let shared = shared.clone();
                AppBarButton::new(
                    label,
                    Listener::new(move |_| shared.clicks.borrow_mut().push(label)),
                )
                .icon(FontIcon::symbol(FluentSymbol::Add))
                .dynamic_overflow_order(order)
                .into()
            })
            .collect();
        let toggled = shared.clone();
        let mut secondary = vec![];
        if setup.secondary {
            secondary.push(
                AppBarToggleButton::new("Pinned", shared.checked.get(), move |app, value| {
                    toggled.checked.set(value);
                    self.set_state(app, |_| {});
                })
                .is_three_state(shared.three_state.get())
                .into(),
            );
        }
        let opened = shared.clone();
        let overflow = shared.clone();
        let bar = CommandBar::new(shared.open.get(), move |app, open| {
            opened.open.set(open);
            self.set_state(app, |_| {});
        })
        .primary_commands(primary)
        .secondary_commands(secondary)
        .default_label_position(setup.label_position)
        .is_sticky(setup.sticky)
        .opening(log("opening"))
        .opened(log("opened"))
        .closing(log("closing"))
        .closed(log("closed"))
        .dynamic_overflow_items_changing(move |_, args| {
            overflow.overflow.borrow_mut().push(args.action)
        });
        Align::new()
            .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
            .child(
                Column::new()
                    .main_axis_size(waft_rendering::MainAxisSize::Min)
                    .cross_axis_alignment(waft_rendering::CrossAxisAlignment::Start)
                    .children([
                        SizedBox::new()
                            .width(shared.width.get())
                            .child(bar)
                            .into_widget(),
                        Text::new("Below the bar").into_widget(),
                    ]),
            )
            .into_widget()
    }
}

fn fixture(setup: Setup) -> (Fixture, Rc<Shared>) {
    let shared = Rc::new(Shared::default());
    shared.width.set(setup.width);
    SHARED.with(|slot| *slot.borrow_mut() = Some(shared.clone()));
    let f = Fixture::with_root([800, 600], move |app| {
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
                    Directionality::new(
                        if setup.rtl {
                            waft_embedder::TextDirection::Rtl
                        } else {
                            waft_embedder::TextDirection::Ltr
                        },
                        ColoredBox::new(Color::from_argb(255, 255, 255, 255)).child(Host { setup }),
                    ),
                ))
                .into_widget(),
        );
    });
    let mut f = f;
    settle(&mut f);
    (f, shared)
}

fn settle(f: &mut Fixture) {
    for _ in 0..20 {
        f.frame_after(Duration::from_millis(20));
    }
}

fn rebuild(f: &mut Fixture, shared: &Shared) {
    let host = shared.host.get().unwrap();
    host.set_state(&mut f.cell.borrow_mut(), |_| {});
    f.cell.checkpoint();
    settle(f);
}

/// The global rectangles of the labels reading `text`.
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
            // A text element and the paragraph under it share the render object.
            if !found.contains(&rect) {
                found.push(rect);
            }
        }
    }
    found
}

/// The commands the open overflow menu lists, top to bottom: the labels below the bar's closed height.
fn overflow_items(f: &Fixture) -> Vec<&'static str> {
    let mut items: Vec<(f64, &'static str)> = LABELS
        .iter()
        .chain(["Pinned"].iter())
        .flat_map(|&label| {
            labels(f, label)
                .into_iter()
                .filter(|rect| rect.top >= 48.0)
                .map(move |rect| (rect.top, label))
        })
        .collect();
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    items.into_iter().map(|(_, label)| label).collect()
}

fn tap_at(f: &mut Fixture, point: Offset) {
    f.send(PointerChange::Down, point);
    f.at += Duration::from_millis(20);
    f.send(PointerChange::Up, point);
    settle(f);
}

/// The `MoreButton` sits at the bar's end, in its top 48 pixels.
fn more_button(shared: &Shared) -> Offset {
    Offset::new(shared.width.get() - 24.0, 24.0)
}

/// The centre of the `index`th of the six primary commands, all in the bar against the `MoreButton`, in its icon's row.
fn command(shared: &Shared, index: usize) -> Offset {
    let start = shared.width.get() - 48.0 - 68.0 * LABELS.len() as f64;
    Offset::new(start + 68.0 * index as f64 + 34.0, 24.0)
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

/// The label of the command whose button has the focus.
fn focused(f: &Fixture) -> Option<String> {
    let focused = primary_focus(&mut f.cell.borrow_mut())?;
    for text in LABELS.iter().chain(["Pinned"].iter()) {
        let elements = f.elements();
        let mut app = f.cell.borrow_mut();
        for element in elements {
            if let Some(object) = element.render_object(&app)
                && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
                && paragraph.text(&app).to_plain_text(true, true) == *text
                && Focus::maybe_of(&mut app, element, false, false) == Some(focused)
            {
                return Some(text.to_string());
            }
        }
    }
    None
}

#[test]
fn commands_that_do_not_fit_move_into_the_overflow_and_come_back() {
    let (mut f, shared) = fixture(Setup {
        width: 300.0,
        ..Setup::default()
    });
    assert_eq!(
        *shared.overflow.borrow(),
        vec![CommandBarDynamicOverflowAction::AddingToOverflow]
    );
    tap_at(&mut f, more_button(&shared));
    assert!(shared.open.get());
    assert_eq!(overflow_items(&f), vec!["Delta", "Echo", "Foxtrot"]);
    f.capture("command_bar_overflow_open");
    key(&mut f, Physical::ESCAPE, Key::ESCAPE);
    assert!(!shared.open.get());

    shared.overflow.borrow_mut().clear();
    shared.width.set(640.0);
    rebuild(&mut f, &shared);
    assert_eq!(
        *shared.overflow.borrow(),
        vec![CommandBarDynamicOverflowAction::RemovingFromOverflow]
    );
    tap_at(&mut f, more_button(&shared));
    assert!(overflow_items(&f).is_empty());
}

#[test]
fn the_lowest_dynamic_overflow_order_moves_first() {
    let (mut f, shared) = fixture(Setup {
        width: 300.0,
        orders: [2, 0, 1, 0, 0, 0],
        ..Setup::default()
    });
    tap_at(&mut f, more_button(&shared));
    // Charlie, then Alpha, then from the right.
    assert_eq!(overflow_items(&f), vec!["Alpha", "Charlie", "Foxtrot"]);
}

#[test]
fn moved_commands_come_before_the_secondary_ones() {
    let (mut f, shared) = fixture(Setup {
        width: 300.0,
        secondary: true,
        ..Setup::default()
    });
    f.capture("command_bar_closed");
    tap_at(&mut f, more_button(&shared));
    assert_eq!(
        overflow_items(&f),
        vec!["Delta", "Echo", "Foxtrot", "Pinned"]
    );
    f.capture("command_bar_open_with_secondary");
}

#[test]
fn bottom_labels_show_only_while_the_bar_is_open() {
    let (mut f, shared) = fixture(Setup::default());
    assert!(labels(&f, "Alpha").is_empty());
    tap_at(&mut f, more_button(&shared));
    let label = labels(&f, "Alpha")[0];
    // Below the icon, inside the bar's 64 pixels.
    assert!(label.top > 32.0 && label.bottom < 64.0, "{label:?}");
    assert_eq!(shared.events.borrow().clone(), vec!["opening", "opened"]);
    key(&mut f, Physical::ESCAPE, Key::ESCAPE);
    assert!(labels(&f, "Alpha").is_empty());
}

#[test]
fn right_labels_sit_beside_the_icon_and_collapsed_ones_are_not_shown() {
    let (f, _) = fixture(Setup {
        label_position: CommandBarDefaultLabelPosition::Right,
        ..Setup::default()
    });
    f.capture("command_bar_right_labels");
    let label = labels(&f, "Alpha")[0];
    let bravo = labels(&f, "Bravo")[0];
    assert!(label.left > 4.0 + 12.0 + 16.0, "{label:?}");
    assert!(label.top < 24.0 && label.bottom > 24.0, "{label:?}");
    // The button is as wide as its label: Bravo follows Alpha's label.
    assert!(bravo.left > label.right, "{bravo:?}");

    let (f, _) = fixture(Setup {
        label_position: CommandBarDefaultLabelPosition::Collapsed,
        ..Setup::default()
    });
    assert!(labels(&f, "Alpha").is_empty());
}

#[test]
fn right_to_left_mirrors_the_bar_and_its_overflow() {
    let (mut f, shared) = fixture(Setup {
        width: 300.0,
        rtl: true,
        ..Setup::default()
    });
    // The `MoreButton` at the bar's end, now the left, the commands beside it to its right.
    tap_at(&mut f, Offset::new(24.0, 24.0));
    assert!(shared.open.get());
    let alpha = labels(&f, "Alpha")[0];
    assert!(alpha.left > 150.0, "{alpha:?}");
    // The menu aligns with the bar's end edge, the left.
    let delta = labels(&f, "Delta")[0];
    assert!(delta.left < 150.0, "{delta:?}");
}

#[test]
fn a_toggle_button_cycles_its_checked_state() {
    let (mut f, shared) = fixture(Setup {
        secondary: true,
        ..Setup::default()
    });
    shared.checked.set(Some(false));
    rebuild(&mut f, &shared);
    tap_at(&mut f, more_button(&shared));
    let pinned = labels(&f, "Pinned")[0].center();
    tap_at(&mut f, pinned);
    assert_eq!(shared.checked.get(), Some(true));
    // A toggle closes the bar.
    assert!(!shared.open.get());

    shared.three_state.set(true);
    rebuild(&mut f, &shared);
    tap_at(&mut f, more_button(&shared));
    let pinned = labels(&f, "Pinned")[0].center();
    tap_at(&mut f, pinned);
    assert_eq!(shared.checked.get(), None);
    tap_at(&mut f, more_button(&shared));
    let pinned = labels(&f, "Pinned")[0].center();
    tap_at(&mut f, pinned);
    assert_eq!(shared.checked.get(), Some(false));
}

#[test]
fn a_command_closes_the_bar_and_runs() {
    let (mut f, shared) = fixture(Setup::default());
    tap_at(&mut f, more_button(&shared));
    tap_at(&mut f, command(&shared, 1));
    assert_eq!(*shared.clicks.borrow(), vec!["Bravo"]);
    assert!(!shared.open.get());
    assert_eq!(
        shared.events.borrow().clone(),
        vec!["opening", "opened", "closing", "closed"]
    );
}

#[test]
fn opened_and_closed_follow_the_transitions() {
    let (mut f, shared) = fixture(Setup::default());
    let point = more_button(&shared);
    f.send(PointerChange::Down, point);
    f.at += Duration::from_millis(20);
    f.send(PointerChange::Up, point);
    for _ in 0..5 {
        f.frame_after(Duration::from_millis(20));
    }
    assert_eq!(shared.events.borrow().clone(), vec!["opening"]);
    for _ in 0..15 {
        f.frame_after(Duration::from_millis(20));
    }
    assert_eq!(shared.events.borrow().clone(), vec!["opening", "opened"]);

    let point = more_button(&shared);
    f.send(PointerChange::Down, point);
    f.at += Duration::from_millis(20);
    f.send(PointerChange::Up, point);
    for _ in 0..4 {
        f.frame_after(Duration::from_millis(20));
    }
    assert_eq!(
        shared.events.borrow().clone(),
        vec!["opening", "opened", "closing"]
    );
    settle(&mut f);
    assert_eq!(
        shared.events.borrow().clone(),
        vec!["opening", "opened", "closing", "closed"]
    );
}

#[test]
fn a_press_outside_closes_the_bar_unless_it_is_sticky() {
    let (mut f, shared) = fixture(Setup::default());
    tap_at(&mut f, more_button(&shared));
    tap_at(&mut f, Offset::new(400.0, 400.0));
    assert!(!shared.open.get());

    let (mut f, shared) = fixture(Setup {
        sticky: true,
        ..Setup::default()
    });
    tap_at(&mut f, more_button(&shared));
    tap_at(&mut f, Offset::new(400.0, 400.0));
    assert!(shared.open.get());
}

#[test]
fn arrow_keys_move_along_the_bar_and_into_the_overflow() {
    let (mut f, shared) = fixture(Setup {
        label_position: CommandBarDefaultLabelPosition::Right,
        width: 360.0,
        secondary: true,
        ..Setup::default()
    });
    f.focus("Alpha");
    key(&mut f, Physical::ARROW_RIGHT, Key::ARROW_RIGHT);
    assert_eq!(focused(&f).as_deref(), Some("Bravo"));
    key(&mut f, Physical::ARROW_LEFT, Key::ARROW_LEFT);
    assert_eq!(focused(&f).as_deref(), Some("Alpha"));
    // Left does not wrap.
    key(&mut f, Physical::ARROW_LEFT, Key::ARROW_LEFT);
    assert_eq!(focused(&f).as_deref(), Some("Alpha"));
    for _ in 0..8 {
        key(&mut f, Physical::ARROW_RIGHT, Key::ARROW_RIGHT);
    }
    // On the `MoreButton`, Down opens the menu with its first item focused.
    assert!(focused(&f).is_none());
    key(&mut f, Physical::ARROW_DOWN, Key::ARROW_DOWN);
    assert!(shared.open.get());
    let items = overflow_items(&f);
    assert_eq!(focused(&f).as_deref(), Some(items[0]));
    // Up from the first item wraps to the `MoreButton`.
    key(&mut f, Physical::ARROW_UP, Key::ARROW_UP);
    assert!(focused(&f).is_none());
    key(&mut f, Physical::ARROW_UP, Key::ARROW_UP);
    assert_eq!(focused(&f).as_deref(), Some("Pinned"));
    key(&mut f, Physical::ESCAPE, Key::ESCAPE);
    assert!(!shared.open.get());
    assert!(focused(&f).is_none());
}

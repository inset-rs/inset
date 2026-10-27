//! Navigation's private attachment through the native overlay, focus and GPU pipeline.
#![feature(arbitrary_self_types)]
#[path = "../../../crates/inset-winui/src/primitives/anchored_flyout.rs"]
mod attachment;
mod theme {}
use attachment::{AnchoredFlyout, AnchoredFlyoutPlacement};
use inset_embedder::{Offset, PointerChange, TextDirection};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::{CrossAxisAlignment, MainAxisSize};
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, Default)]
struct Probe {
    state: Rc<Cell<Option<Handle<PageState>>>>,
    opener: Rc<Cell<Option<AnyFocusNode>>>,
    first: Rc<Cell<Option<AnyFocusNode>>>,
    last: Rc<Cell<Option<AnyFocusNode>>>,
}
#[derive(Debug)]
struct Page(Probe);
struct PageState {
    state: StateData<Page>,
    open: bool,
    dark: bool,
    right: bool,
    leading: bool,
    rtl: bool,
    bottom: bool,
    long: bool,
    nested: bool,
    child_open: bool,
    dismissed: usize,
    clicks: usize,
}
impl StatefulWidget for Page {
    type State = PageState;
    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            open: false,
            dark: false,
            right: false,
            leading: false,
            rtl: false,
            bottom: false,
            long: false,
            nested: false,
            child_open: false,
            dismissed: 0,
            clicks: 0,
        }
    }
}
impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();
    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let p = self.widget(app).0.clone();
        p.state.set(Some(self));
        let anchor = Button::text(
            "Open",
            Listener::new(move |app| self.set_state(app, |s| s.open = true)),
        )
        .focus_node(p.opener.get().unwrap());
        let first = Button::text(
            "First",
            Listener::new(move |app| self.set_state(app, |s| s.clicks += 1)),
        )
        .focus_node(p.first.get().unwrap());
        let last = Button::text("Last", Listener::new(|_| {})).focus_node(p.last.get().unwrap());
        let mut items = vec![first.into_widget()];
        if app.get(self).long {
            for n in 0..30 {
                items.push(Button::text(format!("Item {n}"), Listener::new(|_| {})).into_widget());
            }
        }
        items.push(last.into_widget());
        if app.get(self).nested {
            items = vec![
                AnchoredFlyout::new(
                    app.get(self).child_open,
                    Button::text(
                        "Child",
                        Listener::new(move |app| self.set_state(app, |s| s.child_open = true)),
                    )
                    .focus_node(p.first.get().unwrap()),
                    Button::text("Leaf", Listener::new(|_| {})).focus_node(p.last.get().unwrap()),
                    Listener::new(move |app| self.set_state(app, |s| s.child_open = false)),
                )
                .placement(AnchoredFlyoutPlacement::RightEdgeAlignedTop)
                .into_widget(),
            ];
        }
        let content = SizedBox::new().width(160.0).child(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children(items),
        );
        let popup = AnchoredFlyout::new(
            app.get(self).open,
            anchor,
            content,
            Listener::new(move |app| {
                self.set_state(app, |s| {
                    s.open = false;
                    s.dismissed += 1;
                })
            }),
        )
        .placement(if app.get(self).right {
            AnchoredFlyoutPlacement::RightEdgeAlignedTop
        } else if app.get(self).leading {
            AnchoredFlyoutPlacement::BottomEdgeAlignedLeft
        } else {
            AnchoredFlyoutPlacement::BottomEdgeAlignedRight
        })
        .padding([4.0; 4])
        .offset(if app.get(self).right {
            Offset::new(0.0, -4.0)
        } else {
            Offset::ZERO
        })
        .max_width(300.0);
        ThemeScope::new(
            if app.get(self).dark {
                Theme::Dark
            } else {
                Theme::Light
            },
            ColoredBox::new(
                ThemeResources::new(
                    if app.get(self).dark {
                        Theme::Dark
                    } else {
                        Theme::Light
                    },
                    AccentPalette::default(),
                )
                .solid_background_fill_color_base,
            )
            .child(Stack::new().children(vec![
                    Positioned::new(Directionality::new(
                        if app.get(self).rtl {
                            TextDirection::Rtl
                        } else {
                            TextDirection::Ltr
                        },
                        popup,
                    ))
                    .left(if app.get(self).right { 440.0 } else { 200.0 })
                    .top(if app.get(self).bottom { 310.0 } else { 30.0 })
                    .into_widget(),
                ])),
        )
        .into_widget()
    }
}
fn fixture() -> (Fixture, Probe) {
    let p = Probe::default();
    let q = p.clone();
    let f = Fixture::with_root([520, 360], move |app| {
        winui_gallery::install_fonts(app);
        q.opener.set(Some(FocusNode::new(app).as_node()));
        q.first.set(Some(FocusNode::new(app).as_node()));
        q.last.set(Some(FocusNode::new(app).as_node()));
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(q.clone()).into_widget()),
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
    (f, p)
}
fn settle(f: &mut Fixture) {
    for _ in 0..4 {
        f.pump();
    }
}
fn update(f: &mut Fixture, p: &Probe, change: impl FnOnce(&mut PageState)) {
    let mut app = f.cell.borrow_mut();
    p.state.get().unwrap().set_state(&mut app, change);
    drop(app);
    settle(f);
}
fn key(f: &mut Fixture, logical: LogicalKeyboardKey, physical: PhysicalKeyboardKey) {
    for e in [
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
        KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)),
    ] {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &e);
        drop(app);
        f.cell.checkpoint();
    }
    settle(f);
}
#[test]
fn focus_cycles_inside_interactive_popup_and_escape_restores_opener() {
    let (mut f, p) = fixture();
    {
        let mut app = f.cell.borrow_mut();
        p.opener.get().unwrap().request_focus(&mut app, None);
    }
    settle(&mut f);
    f.tap("Open");
    settle(&mut f);
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), p.first.get());
    key(&mut f, LogicalKeyboardKey::TAB, PhysicalKeyboardKey::TAB);
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), p.last.get());
    key(&mut f, LogicalKeyboardKey::TAB, PhysicalKeyboardKey::TAB);
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), p.first.get());
    f.tap("First");
    settle(&mut f);
    assert_eq!(f.cell.borrow().get(p.state.get().unwrap()).clicks, 1);
    assert!(f.cell.borrow().get(p.state.get().unwrap()).open);
    f.capture("flyout-light");
    key(
        &mut f,
        LogicalKeyboardKey::ESCAPE,
        PhysicalKeyboardKey::ESCAPE,
    );
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), p.opener.get());
    assert_eq!(f.cell.borrow().get(p.state.get().unwrap()).dismissed, 1);
}
#[test]
fn edge_placement_light_dismiss_and_long_content_scrolling() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| {
        s.right = true;
        s.open = true;
        s.dark = true;
    });
    let first = f.find("First");
    let anchor = f.find("Open");
    assert!(first.dx() < anchor.dx(), "right edge must flip left");
    f.capture("flyout-dark-right-edge");
    f.send(PointerChange::Down, Offset::new(10.0, 340.0));
    f.send(PointerChange::Up, Offset::new(10.0, 340.0));
    settle(&mut f);
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).open);
    update(&mut f, &p, |s| {
        s.right = false;
        s.bottom = true;
        s.long = true;
        s.open = true;
    });
    assert!(f.find("First").dy() < f.find("Open").dy());
    for _ in 0..31 {
        key(&mut f, LogicalKeyboardKey::TAB, PhysicalKeyboardKey::TAB);
    }
    for _ in 0..20 {
        f.pump();
    }
    let last = f.find("Last");
    assert!(
        last.dy() >= 0.0 && last.dy() < 310.0,
        "focus should reveal last item: {last:?}"
    );
    f.capture("flyout-long-focus-scroll");
    update(&mut f, &p, |s| s.open = false);
    assert_eq!(
        f.cell.borrow().get(p.state.get().unwrap()).dismissed,
        1,
        "explicit close is not a dismissal request"
    );
}

#[test]
fn nested_escape_closes_only_top_flyout_and_restores_each_opener() {
    let (mut f, p) = fixture();
    {
        let mut app = f.cell.borrow_mut();
        p.opener.get().unwrap().request_focus(&mut app, None);
    }
    update(&mut f, &p, |s| {
        s.nested = true;
        s.open = true;
    });
    f.tap("Child");
    settle(&mut f);
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), p.last.get());
    key(
        &mut f,
        LogicalKeyboardKey::ESCAPE,
        PhysicalKeyboardKey::ESCAPE,
    );
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).child_open);
    assert!(f.cell.borrow().get(p.state.get().unwrap()).open);
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), p.first.get());
    key(
        &mut f,
        LogicalKeyboardKey::ESCAPE,
        PhysicalKeyboardKey::ESCAPE,
    );
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).open);
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), p.opener.get());
}

#[test]
fn bottom_alignment_distinguishes_overflow_from_children_and_mirrors_rtl() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| s.open = true);
    assert!(f.find("First").dx() < f.find("Open").dx());
    update(&mut f, &p, |s| s.leading = true);
    assert!(f.find("First").dx() > f.find("Open").dx());
    update(&mut f, &p, |s| s.rtl = true);
    assert!(f.find("First").dx() < f.find("Open").dx());
}

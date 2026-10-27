//! SplitView through a real layout, input, focus, animation and GPU pipeline.
#![feature(arbitrary_self_types)]

use inset_embedder::{Offset, PointerChange, Rect};
use inset_foundation::{App, Handle, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::{CrossAxisAlignment, MainAxisSize, RenderParagraph};
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[derive(Clone, Debug, Default)]
struct Probe {
    state: Rc<Cell<Option<Handle<PageState>>>>,
    events: Rc<RefCell<Vec<&'static str>>>,
    cancel: Rc<Cell<bool>>,
    content_clicks: Rc<Cell<u32>>,
    pane_mounts: Rc<Cell<u32>>,
}

#[derive(Debug)]
struct Page(Probe);
struct PageState {
    state: StateData<Page>,
    open: bool,
    mode: SplitViewDisplayMode,
    placement: SplitViewPanePlacement,
    theme: Theme,
    length: f64,
    inset: f64,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            open: false,
            mode: SplitViewDisplayMode::Overlay,
            placement: SplitViewPanePlacement::Left,
            theme: Theme::Light,
            length: 200.0,
            inset: 0.0,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();
    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let probe = self.widget(app).0.clone();
        probe.state.set(Some(self));
        let s = app.get(self);
        let (open, mode, placement, theme, length, inset) =
            (s.open, s.mode, s.placement, s.theme, s.length, s.inset);
        let opening = probe.events.clone();
        let opened = probe.events.clone();
        let closing = probe.events.clone();
        let closed = probe.events.clone();
        let cancel = probe.cancel.clone();
        let changes = probe.events.clone();
        let clicks = probe.content_clicks.clone();
        let resources = ThemeResources::new(theme, AccentPalette::default());
        ThemeScope::new(
            theme,
            ColoredBox::new(resources.solid_background_fill_color_base).child(
                Padding::new(EdgeInsetsGeometry::all(inset)).child(
                    SplitView::new(open, move |app, value| {
                        changes.borrow_mut().push(if value {
                            "changed-open"
                        } else {
                            "changed-closed"
                        });
                        self.set_state(app, |s| s.open = value);
                    })
                    .display_mode(mode)
                    .pane_placement(placement)
                    .open_pane_length(length)
                    .compact_pane_length(48.0)
                    .light_dismiss_overlay_mode(LightDismissOverlayMode::On)
                    .pane_opening(Listener::new(move |_| opening.borrow_mut().push("opening")))
                    .pane_opened(Listener::new(move |_| opened.borrow_mut().push("opened")))
                    .pane_closing(move |_, args| {
                        closing.borrow_mut().push("closing");
                        args.cancel = cancel.get();
                    })
                    .pane_closed(Listener::new(move |_| closed.borrow_mut().push("closed")))
                    .pane(PaneCounter(probe.pane_mounts.clone()))
                    .content(
                        Column::new()
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .children(vec![
                                Text::new("Content bounds")
                                    .style(
                                        TextBlockStyle::Body.text_style(
                                            &resources,
                                            resources.text_fill_color_primary,
                                        ),
                                    )
                                    .into_widget(),
                                Button::text(
                                    "Content action",
                                    Listener::new(move |_| clicks.set(clicks.get() + 1)),
                                )
                                .into_widget(),
                            ]),
                    ),
                ),
            ),
        )
        .into_widget()
    }
}

#[derive(Debug)]
struct PaneCounter(Rc<Cell<u32>>);
struct PaneCounterState {
    state: StateData<PaneCounter>,
    count: u32,
}

impl StatefulWidget for PaneCounter {
    type State = PaneCounterState;

    fn create_state(&self) -> PaneCounterState {
        self.0.set(self.0.get() + 1);
        PaneCounterState {
            state: StateData::new(),
            count: 0,
        }
    }
}

impl State for PaneCounterState {
    type Widget = PaneCounter;
    inset_widgets::state_accessors!();
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let count = app.get(self).count;
        SizedBox::new()
            .width(160.0)
            .child(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children(vec![
                        Text::new("Pane bounds")
                            .style(
                                TextBlockStyle::Body
                                    .text_style(&resources, resources.text_fill_color_primary),
                            )
                            .into_widget(),
                        Button::text(
                            format!("Pane count {count}"),
                            Listener::new(move |app| self.set_state(app, |s| s.count += 1)),
                        )
                        .into_widget(),
                    ]),
            )
            .into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    let probe = Probe::default();
    let page_probe = probe.clone();
    let fixture = Fixture::with_root([640, 280], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(page_probe.clone()).into_widget()),
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
    (fixture, probe)
}

fn update(f: &mut Fixture, probe: &Probe, change: impl FnOnce(&mut PageState)) {
    probe
        .state
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), change);
    f.cell.checkpoint();
}

fn settle(f: &mut Fixture) {
    for _ in 0..3 {
        f.pump();
    }
}

fn frames(f: &mut Fixture, count: u32) {
    for _ in 0..count {
        f.frame_after(Duration::from_millis(20));
    }
}

fn bounds(f: &Fixture, label: &str) -> Rect {
    let elements = f.elements();
    let app = f.cell.borrow();
    for e in elements {
        if let Some(o) = e.render_object(&app)
            && let Some(p) = o.downcast::<RenderParagraph>(&app)
            && p.text(&app).to_plain_text(true, true) == label
        {
            let b = o.as_box().unwrap();
            let origin = b.local_to_global(&app, Offset::ZERO, None);
            let size = b.size(&app);
            return Rect::from_ltwh(origin.dx(), origin.dy(), size.width(), size.height());
        }
    }
    panic!("missing {label}");
}

fn assert_content(f: &Fixture, left: f64, width: f64) {
    let rect = bounds(f, "Content bounds");
    assert!(
        (rect.left - left).abs() < 0.1,
        "{rect:?}, expected left {left}"
    );
    assert!(
        (rect.width() - width).abs() < 0.1,
        "{rect:?}, expected width {width}"
    );
}

fn key(f: &mut Fixture, logical: LogicalKeyboardKey, physical: PhysicalKeyboardKey) {
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

#[test]
fn all_modes_and_placements_allocate_content_and_render_both_themes() {
    let (mut f, p) = fixture();
    for placement in [SplitViewPanePlacement::Left, SplitViewPanePlacement::Right] {
        for mode in [
            SplitViewDisplayMode::Overlay,
            SplitViewDisplayMode::Inline,
            SplitViewDisplayMode::CompactOverlay,
            SplitViewDisplayMode::CompactInline,
        ] {
            update(&mut f, &p, |s| {
                s.open = false;
                s.mode = mode;
                s.placement = placement;
            });
            settle(&mut f);
            let compact = matches!(
                mode,
                SplitViewDisplayMode::CompactOverlay | SplitViewDisplayMode::CompactInline
            );
            let reserve = if compact { 48.0 } else { 0.0 };
            assert_content(
                &f,
                if placement == SplitViewPanePlacement::Left {
                    reserve
                } else {
                    0.0
                },
                640.0 - reserve,
            );
            update(&mut f, &p, |s| s.open = true);
            frames(&mut f, 4);
            f.capture(&format!("split_view_{mode:?}_{placement:?}_opening"));
            settle(&mut f);
            let reserve = match mode {
                SplitViewDisplayMode::Inline | SplitViewDisplayMode::CompactInline => 200.0,
                SplitViewDisplayMode::CompactOverlay => 48.0,
                _ => 0.0,
            };
            assert_content(
                &f,
                if placement == SplitViewPanePlacement::Left {
                    reserve
                } else {
                    0.0
                },
                640.0 - reserve,
            );
            let pane = bounds(&f, "Pane bounds");
            assert!(
                (pane.left
                    - if placement == SplitViewPanePlacement::Left {
                        0.0
                    } else {
                        440.0
                    })
                .abs()
                    < 0.1,
                "{pane:?}"
            );
            for theme in [Theme::Light, Theme::Dark] {
                update(&mut f, &p, |s| s.theme = theme);
                settle(&mut f);
                f.capture(&format!("split_view_{mode:?}_{placement:?}_{theme:?}"));
            }
        }
    }
}

#[test]
fn canceled_light_dismiss_keeps_open_but_explicit_close_cannot_be_canceled() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| s.open = true);
    settle(&mut f);
    p.events.borrow_mut().clear();
    p.cancel.set(true);
    let point = Offset::new(500.0, 180.0);
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    settle(&mut f);
    assert!(f.cell.borrow().get(p.state.get().unwrap()).open);
    assert_eq!(&*p.events.borrow(), &["closing"]);
    update(&mut f, &p, |s| s.open = false);
    settle(&mut f);
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).open);
    assert!(p.events.borrow().contains(&"closed"));
    p.cancel.set(false);
    update(&mut f, &p, |s| s.open = true);
    settle(&mut f);
    p.events.borrow_mut().clear();
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    settle(&mut f);
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).open);
    assert_eq!(
        &*p.events.borrow(),
        &["closing", "changed-closed", "closed"]
    );
}

#[test]
fn pane_state_survives_collapsing_and_auto_length_measures_its_child() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| {
        s.open = true;
        s.mode = SplitViewDisplayMode::Inline;
    });
    settle(&mut f);
    f.tap("Pane count 0");
    f.find("Pane count 1");
    let mounts = p.pane_mounts.get();
    update(&mut f, &p, |s| s.open = false);
    settle(&mut f);
    update(&mut f, &p, |s| {
        s.open = true;
        s.length = f64::NAN;
    });
    settle(&mut f);
    f.find("Pane count 1");
    assert_eq!(p.pane_mounts.get(), mounts);
    assert_content(&f, 160.0, 480.0);
    f.capture("split_view_auto_length");
}

#[test]
fn overlay_focus_returns_to_content_after_escape() {
    let (mut f, p) = fixture();
    f.tap("Content action");
    f.focus("Content action");
    assert_eq!(p.content_clicks.get(), 1);
    update(&mut f, &p, |s| s.open = true);
    settle(&mut f);
    key(
        &mut f,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
    );
    f.find("Pane count 1");
    assert_eq!(p.content_clicks.get(), 1);
    key(&mut f, LogicalKeyboardKey::TAB, PhysicalKeyboardKey::TAB);
    key(
        &mut f,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
    );
    assert_eq!(
        p.content_clicks.get(),
        1,
        "overlay tab traversal escaped to content"
    );
    f.find("Pane count 2");
    key(
        &mut f,
        LogicalKeyboardKey::ESCAPE,
        PhysicalKeyboardKey::ESCAPE,
    );
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).open);
    key(
        &mut f,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
    );
    assert_eq!(p.content_clicks.get(), 2);
}

#[test]
fn embedded_split_view_dismisses_outside_its_bounds_without_covering_the_pane() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| s.inset = 40.0);
    settle(&mut f);
    update(&mut f, &p, |s| s.open = true);
    settle(&mut f);
    f.tap("Pane count 0");
    f.find("Pane count 1");
    assert!(f.cell.borrow().get(p.state.get().unwrap()).open);
    f.capture("split_view_embedded_overlay");
    p.events.borrow_mut().clear();
    let point = Offset::new(10.0, 10.0);
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    settle(&mut f);
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).open);
    assert_eq!(
        &*p.events.borrow(),
        &["closing", "changed-closed", "closed"]
    );
}

#[test]
fn resizing_an_open_overlay_uses_the_cancelable_dismissal_path() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| s.open = true);
    settle(&mut f);
    p.events.borrow_mut().clear();
    p.cancel.set(true);
    update(&mut f, &p, |s| s.inset = 20.0);
    settle(&mut f);
    assert!(f.cell.borrow().get(p.state.get().unwrap()).open);
    assert_eq!(&*p.events.borrow(), &["closing"]);
    p.events.borrow_mut().clear();
    p.cancel.set(false);
    update(&mut f, &p, |s| s.inset = 30.0);
    settle(&mut f);
    assert!(!f.cell.borrow().get(p.state.get().unwrap()).open);
    assert_eq!(
        &*p.events.borrow(),
        &["closing", "changed-closed", "closed"]
    );
}

#[test]
fn changing_open_display_mode_restores_focus_without_reopening_the_pane() {
    let (mut f, p) = fixture();
    f.tap("Content action");
    f.focus("Content action");
    update(&mut f, &p, |s| s.open = true);
    settle(&mut f);
    p.events.borrow_mut().clear();
    update(&mut f, &p, |s| s.mode = SplitViewDisplayMode::Inline);
    settle(&mut f);
    assert!(f.cell.borrow().get(p.state.get().unwrap()).open);
    assert!(
        p.events.borrow().is_empty(),
        "mode change raised pane lifecycle events: {:?}",
        p.events.borrow()
    );
    key(
        &mut f,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
    );
    assert_eq!(p.content_clicks.get(), 2);
}

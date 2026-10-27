//! Adaptive and top navigation use the real native layout, input and overlay stack.
#![feature(arbitrary_self_types)]
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, Default)]
struct Probe(Rc<Cell<Option<Handle<PageState>>>>);

#[derive(Debug)]
struct Page(Probe);

struct PageState {
    state: StateData<Page>,
    width: f64,
    mode: NavigationViewPaneDisplayMode,
    selected: Option<String>,
    theme: Theme,
    item_count: usize,
    cancel_close: bool,
    closing_count: usize,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            width: 1100.0,
            mode: NavigationViewPaneDisplayMode::Auto,
            selected: Some("0".to_owned()),
            theme: Theme::Light,
            item_count: 8,
            cancel_close: false,
            closing_count: 0,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.0.set(Some(self));
        let state = app.get(self);
        let mut nav = NavigationView::new(
            (0..state.item_count)
                .map(|i| {
                    NavigationViewItem::text(i.to_string(), format!("Destination {i}"))
                        .icon(FontIcon::symbol(FluentSymbol::Settings))
                })
                .collect(),
            state.selected.clone(),
            move |app, args| self.set_state(app, |s| s.selected = args.item.map(|item| item.id)),
            Center::new().child(Content(format!(
                "Content {}",
                state.selected.as_deref().unwrap_or("none")
            ))),
        );
        nav.pane_display_mode = state.mode;
        nav.pane_closing = Some(Rc::new(move |app, args| {
            args.cancel = app.get(self).cancel_close;
            app.get_mut(self).closing_count += 1;
        }));
        nav.header = Some(Text::new("Navigation example").into_widget());
        nav.pane_title = "Library".to_owned();
        nav.is_back_button_visible = NavigationViewBackButtonVisible::Collapsed;
        ThemeScope::new(
            state.theme,
            Align::new()
                .alignment(inset_painting::Alignment::TOP_LEFT.into())
                .child(SizedBox::new().width(state.width).height(440.0).child(nav)),
        )
        .into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    let p = Probe::default();
    let probe = p.clone();
    let f = Fixture::with_root([1200, 460], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(probe.clone()).into_widget()),
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

fn update(f: &mut Fixture, p: &Probe, change: impl FnOnce(&mut PageState)) {
    p.0.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), change);
    for _ in 0..3 {
        f.pump();
    }
}

fn split(f: &Fixture) -> (SplitViewDisplayMode, bool) {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements
        .into_iter()
        .find_map(|element| {
            let widget = element.widget(&app);
            downcast_widget::<SplitView>(&**widget)
                .map(|split| (split.display_mode, split.is_pane_open))
        })
        .unwrap()
}

#[test]
fn adaptive_thresholds_and_force_closed_pane() {
    let (mut f, p) = fixture();
    f.pump();
    assert_eq!(split(&f), (SplitViewDisplayMode::CompactInline, true));
    f.capture("navigation_expanded_light");
    update(&mut f, &p, |s| s.width = 1007.0);
    assert_eq!(split(&f), (SplitViewDisplayMode::CompactOverlay, false));
    update(&mut f, &p, |s| s.width = 641.0);
    assert_eq!(split(&f), (SplitViewDisplayMode::CompactOverlay, false));
    f.capture("navigation_compact_light");
    update(&mut f, &p, |s| s.width = 640.0);
    assert_eq!(split(&f), (SplitViewDisplayMode::Overlay, false));
    update(&mut f, &p, |s| {
        s.width = 1100.0;
        s.theme = Theme::Dark;
    });
    assert_eq!(split(&f), (SplitViewDisplayMode::CompactInline, true));
    f.capture("navigation_expanded_dark");
    f.send(
        inset_embedder::PointerChange::Down,
        inset_embedder::Offset::new(24.0, 24.0),
    );
    f.send(
        inset_embedder::PointerChange::Up,
        inset_embedder::Offset::new(24.0, 24.0),
    );
    f.pump();
    assert!(!split(&f).1);
    update(&mut f, &p, |s| s.width = 1150.0);
    assert!(!split(&f).1, "explicit close must survive Expanded resize");
}

#[test]
fn top_overflow_selection_promotes_and_resize_recovers() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| {
        s.mode = NavigationViewPaneDisplayMode::Top;
        s.width = 620.0;
    });
    f.capture("navigation_top_light");
    // The overflow button is the More FontIcon, identified independently of tooltip timing.
    let point = {
        let elements = f.elements();
        let app = f.cell.borrow();
        elements
            .into_iter()
            .find_map(|e| {
                let widget = e.widget(&app);
                let icon = downcast_widget::<FontIcon>(&**widget)?;
                if icon.glyph != FluentSymbol::More.glyph().to_string() {
                    return None;
                }
                let b = e.find_render_object(&app)?.as_box()?;
                Some(
                    b.local_to_global(&app, inset_embedder::Offset::ZERO, None)
                        + inset_embedder::Offset::new(
                            b.size(&app).width() / 2.0,
                            b.size(&app).height() / 2.0,
                        ),
                )
            })
            .unwrap()
    };
    f.send(inset_embedder::PointerChange::Down, point);
    f.send(inset_embedder::PointerChange::Up, point);
    f.pump();
    f.tap("Destination 7");
    f.pump();
    assert_eq!(
        f.cell.borrow().get(p.0.get().unwrap()).selected.as_deref(),
        Some("7")
    );
    f.capture("navigation_top_selected_overflow");
    update(&mut f, &p, |s| {
        s.width = 1180.0;
        s.theme = Theme::Dark;
    });
    f.capture("navigation_top_dark");
}

/// Stateful page content detects accidental unmounting while the shell changes orientation.
#[derive(Debug)]
struct Content(String);

struct ContentState {
    state: StateData<Content>,
    clicks: usize,
}

impl StatefulWidget for Content {
    type State = ContentState;

    fn create_state(&self) -> ContentState {
        ContentState {
            state: StateData::new(),
            clicks: 0,
        }
    }
}

impl State for ContentState {
    type Widget = Content;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        Column::new()
            .main_axis_size(inset_rendering::MainAxisSize::Min)
            .children([
                Text::new(self.widget(app).0.clone()).into_widget(),
                Button::text(
                    format!("Page clicks {}", app.get(self).clicks),
                    Listener::new(move |app| self.set_state(app, |s| s.clicks += 1)),
                )
                .into_widget(),
            ])
            .into_widget()
    }
}

#[test]
fn page_state_survives_reparenting_and_removed_selection_clears() {
    let (mut f, p) = fixture();
    f.tap("Page clicks 0");
    update(&mut f, &p, |s| s.mode = NavigationViewPaneDisplayMode::Top);
    f.find("Page clicks 1");
    update(&mut f, &p, |s| s.mode = NavigationViewPaneDisplayMode::Left);
    f.find("Page clicks 1");
    update(&mut f, &p, |s| s.item_count = 0);
    assert_eq!(f.cell.borrow().get(p.0.get().unwrap()).selected, None);
    f.find("Content none");
}

#[test]
fn pane_closing_is_cancelable_only_for_light_dismiss_and_raised_once() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| {
        s.mode = NavigationViewPaneDisplayMode::LeftCompact;
        s.cancel_close = true;
    });
    let toggle = inset_embedder::Offset::new(24.0, 24.0);
    for change in [
        inset_embedder::PointerChange::Down,
        inset_embedder::PointerChange::Up,
    ] {
        f.send(change, toggle);
    }
    f.pump();
    assert!(split(&f).1);
    let before = f.cell.borrow().get(p.0.get().unwrap()).closing_count;
    let outside = inset_embedder::Offset::new(600.0, 200.0);
    for change in [
        inset_embedder::PointerChange::Down,
        inset_embedder::PointerChange::Up,
    ] {
        f.send(change, outside);
    }
    f.pump();
    assert!(split(&f).1);
    assert_eq!(
        f.cell.borrow().get(p.0.get().unwrap()).closing_count,
        before + 1
    );
    update(&mut f, &p, |s| s.cancel_close = false);
    for change in [
        inset_embedder::PointerChange::Down,
        inset_embedder::PointerChange::Up,
    ] {
        f.send(change, outside);
    }
    f.pump();
    assert!(!split(&f).1);
    assert_eq!(
        f.cell.borrow().get(p.0.get().unwrap()).closing_count,
        before + 2
    );
}

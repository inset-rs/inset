//! NavigationView's back, toggle, title and header chrome through GPU layout and input.
#![feature(arbitrary_self_types)]

use inset_embedder::{Offset, Rect};
use inset_foundation::{App, Handle, Listener, ValueKey};
use inset_rendering::{RenderBox, RenderParagraph};
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
    open: bool,
    toggle_visible: bool,
    mode: NavigationViewPaneDisplayMode,
    always_show_header: bool,
    back_count: usize,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> Self::State {
        PageState {
            state: StateData::new(),
            open: false,
            toggle_visible: true,
            mode: NavigationViewPaneDisplayMode::LeftMinimal,
            always_show_header: true,
            back_count: 0,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.0.set(Some(self));
        let state = app.get(self);
        let owner = self;
        let mut nav = NavigationView::new(
            vec![NavigationViewItem::text("home", "Home")],
            Some("home".to_owned()),
            |_, _| {},
            Text::new("Page content"),
        )
        .pane_display_mode(state.mode)
        .is_pane_open(state.open, move |app, open| {
            owner.set_state(app, |state| state.open = open)
        })
        .is_pane_toggle_button_visible(state.toggle_visible)
        .is_back_button_visible(NavigationViewBackButtonVisible::Visible)
        .is_back_enabled(true)
        .always_show_header(state.always_show_header)
        .header(Text::new("Page header"))
        .pane_header(Text::new("Pane header"))
        .pane_title("Pane title");
        nav.back_requested = Some(Listener::new(move |app| {
            owner.set_state(app, |state| state.back_count += 1)
        }));
        ThemeScope::new(
            Theme::Light,
            SizedBox::new().width(800.0).height(400.0).child(nav),
        )
        .into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    let probe = Probe::default();
    let page = probe.clone();
    let f = Fixture::with_root([800, 400], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(page.clone()).into_widget()),
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
    (f, probe)
}

fn update(f: &mut Fixture, probe: &Probe, change: impl FnOnce(&mut PageState)) {
    probe
        .0
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), change);
    for _ in 0..3 {
        f.pump();
    }
}

fn onstage(e: AnyElement, app: &App) -> bool {
    let mut current = Some(e);
    while let Some(element) = current {
        if downcast_widget::<Offstage>(element.widget(app).as_ref())
            .is_some_and(|offstage| offstage.offstage)
        {
            return false;
        }
        current = element.parent(app);
    }
    true
}

fn text_bounds(f: &Fixture, text: &str) -> Option<Rect> {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().find_map(|e| {
        if !onstage(e, &app) {
            return None;
        }
        let paragraph = e.render_object(&app)?.downcast::<RenderParagraph>(&app)?;
        (paragraph.text(&app).to_plain_text(true, true) == text).then(|| {
            let render = paragraph.as_box();
            render.local_to_global(&app, Offset::ZERO, None) & render.size(&app)
        })
    })
}

fn key_bounds(f: &Fixture, value: &'static str) -> Option<Rect> {
    let expected = ValueKey::new(value);
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().find_map(|e| {
        if !onstage(e, &app)
            || !e
                .widget(&app)
                .key()
                .is_some_and(|key| key.as_ref().eq_key(&expected))
        {
            return None;
        }
        let render = e.find_render_object(&app)?.as_box()?;
        Some(render.local_to_global(&app, Offset::ZERO, None) & render.size(&app))
    })
}

fn tap_key(f: &mut Fixture, key: &'static str) {
    let point = key_bounds(f, key)
        .unwrap_or_else(|| panic!("missing onstage keyed control {key}"))
        .center();
    f.send(inset_embedder::PointerChange::Down, point);
    f.send(inset_embedder::PointerChange::Up, point);
    f.pump();
}

#[test]
fn minimal_back_callback_and_close_replaces_toggle() {
    let (mut f, probe) = fixture();
    assert!(key_bounds(&f, "NavigationViewBackButton").is_some());
    tap_key(&mut f, "NavigationViewBackButton");
    assert_eq!(f.cell.borrow().get(probe.0.get().unwrap()).back_count, 1);

    tap_key(&mut f, "TogglePaneButton");
    assert!(f.cell.borrow().get(probe.0.get().unwrap()).open);
    assert!(key_bounds(&f, "NavigationViewCloseButton").is_some());
    assert!(key_bounds(&f, "NavigationViewBackButton").is_none());
    f.capture("navigation-minimal-open-close");

    update(&mut f, &probe, |state| state.toggle_visible = false);
    assert!(key_bounds(&f, "TogglePaneButton").is_none());
    assert!(key_bounds(&f, "NavigationViewCloseButton").is_some());
    tap_key(&mut f, "NavigationViewCloseButton");
    assert!(!f.cell.borrow().get(probe.0.get().unwrap()).open);
    assert_eq!(f.cell.borrow().get(probe.0.get().unwrap()).back_count, 1);
}

#[test]
fn pane_title_toggles_and_pane_header_is_independent() {
    let (mut f, probe) = fixture();
    update(&mut f, &probe, |state| state.open = true);
    assert!(text_bounds(&f, "Pane title").unwrap().width() > 0.0);
    f.tap("Pane title");
    assert!(!f.cell.borrow().get(probe.0.get().unwrap()).open);

    update(&mut f, &probe, |state| {
        state.open = true;
        state.toggle_visible = false;
    });
    assert!(text_bounds(&f, "Pane title").is_some());
    assert!(text_bounds(&f, "Pane header").is_some());
}

#[test]
fn compact_pane_hides_custom_pane_header() {
    let (mut f, probe) = fixture();
    update(&mut f, &probe, |state| {
        state.mode = NavigationViewPaneDisplayMode::LeftCompact;
        state.open = false;
    });
    assert!(text_bounds(&f, "Pane header").is_none());
}

#[test]
fn top_hides_page_header_when_always_show_header_is_false() {
    let (mut f, probe) = fixture();
    update(&mut f, &probe, |state| {
        state.mode = NavigationViewPaneDisplayMode::Top;
        state.always_show_header = false;
    });
    assert!(text_bounds(&f, "Page header").is_none());
    assert!(text_bounds(&f, "Pane title").is_none());
    update(&mut f, &probe, |state| state.toggle_visible = false);
    assert!(text_bounds(&f, "Pane title").is_some());
    assert!(text_bounds(&f, "Pane header").is_some());
    f.capture("navigation-top-title-and-header");

    update(&mut f, &probe, |state| {
        state.mode = NavigationViewPaneDisplayMode::LeftMinimal;
        state.open = true;
    });
    assert!(text_bounds(&f, "Page header").is_some());
}

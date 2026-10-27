//! Custom pane footers measure independently of settings and footer menu items.
#![feature(arbitrary_self_types)]

use inset_embedder::Offset;
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc};

/// Gives the test access to the owning page and its custom footer container.
#[derive(Clone, Debug)]
struct Probe {
    owner: Rc<Cell<Option<Handle<PageState>>>>,
    footer: Rc<GlobalKey>,
}

/// Owns the footer size without adding settings or footer menu entries.
#[derive(Debug)]
struct Page(Probe);

/// State changed by the test and the real footer button.
struct PageState {
    state: StateData<Page>,
    footer_height: f64,
    clicks: usize,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> Self::State {
        PageState {
            state: StateData::new(),
            footer_height: 40.0,
            clicks: 0,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let probe = self.widget(app).0.clone();
        probe.owner.set(Some(self));
        let footer = SizedBox::new()
            .key(probe.footer)
            .height(app.get(self).footer_height)
            .child(Center::new().child(Button::text(
                "Footer action",
                Listener::new(move |app| self.set_state(app, |state| state.clicks += 1)),
            )));
        ThemeScope::new(
            Theme::Light,
            NavigationView::new(
                (0..20)
                    .map(|i| NavigationViewItem::text(i.to_string(), format!("Item {i}")))
                    .collect(),
                Some("0".into()),
                |_, _| {},
                Text::new("Page"),
            )
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .is_pane_open(true, |_, _| {})
            .is_settings_visible(false)
            .pane_footer(footer),
        )
        .into_widget()
    }
}

fn assert_footer_visible(f: &mut Fixture, probe: &Probe, height: f64) {
    let mut app = f.cell.borrow_mut();
    let context = probe.footer.current_context(&mut app).unwrap();
    let render = context.find_render_object(&app).unwrap().as_box().unwrap();
    let origin = render.local_to_global(&app, Offset::ZERO, None);
    let size = render.size(&app);
    assert!(
        (size.height() - height).abs() < 0.01,
        "footer must receive its measured height: {size:?}"
    );
    assert!(
        origin.dy() >= 0.0 && origin.dy() + size.height() <= 400.01,
        "footer must fit inside the pane: {origin:?} {size:?}"
    );
    drop(app);
    let point = f.find("Footer action");
    assert!((0.0..400.0).contains(&point.dy()));
}

#[test]
fn custom_footer_starts_visible_and_reallocates_after_owner_resize() {
    let probe = Probe {
        owner: Rc::new(Cell::new(None)),
        footer: Rc::new(GlobalKey::new()),
    };
    let copy = probe.clone();
    let mut f = Fixture::with_root([800, 400], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(copy.clone()).into_widget()),
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
    assert_footer_visible(&mut f, &probe, 40.0);
    f.tap("Footer action");
    for height in [112.0, 24.0] {
        let owner = probe.owner.get().unwrap();
        owner.set_state(&mut f.cell.borrow_mut(), |state| {
            state.footer_height = height
        });
        for _ in 0..3 {
            f.pump();
        }
        assert_footer_visible(&mut f, &probe, height);
        f.tap("Footer action");
    }
    assert_eq!(f.cell.borrow().get(probe.owner.get().unwrap()).clicks, 3);
}

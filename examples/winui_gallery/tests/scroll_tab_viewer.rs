//! Exercise the private TabView scrolling template without exposing it as public kit API.
#![feature(arbitrary_self_types)]
#[allow(dead_code)]
#[path = "../../../crates/inset-winui/src/controls/scroll_tab_viewer.rs"]
mod template;

use inset_embedder::{Offset, PointerChange, Rect};
use inset_foundation::{App, Handle, Listener};
use inset_painting::{Axis, EdgeInsetsGeometry};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc, time::Duration};
use template::TabScrollViewer;

#[derive(Clone, Debug, Default)]
struct Probe {
    state: Rc<Cell<Option<Handle<PageState>>>>,
    controller: Rc<Cell<Option<Handle<ScrollViewportController>>>>,
    built: Rc<Cell<usize>>,
}
#[derive(Debug)]
struct Page(Probe);
struct PageState {
    state: StateData<Page>,
    show_buttons: bool,
    dark: bool,
}
impl StatefulWidget for Page {
    type State = PageState;
    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            show_buttons: true,
            dark: false,
        }
    }
}
impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let probe = self.widget(app).0.clone();
        probe.state.set(Some(self));
        let controller = probe.controller.get().unwrap();
        let native = controller.native_controller(app);
        let built = probe.built.clone();
        let list = ListView::builder(move |_, _, index| {
            built.set(built.get() + 1);
            Some(Button::text(format!("Tab {index}"), Listener::new(|_| {})).into_widget())
        })
        .item_count(1000)
        .build()
        .item_extent(100.0)
        .scroll_direction(Axis::Horizontal)
        .padding(EdgeInsetsGeometry::ZERO)
        .controller(native);
        let theme = if app.get(self).dark {
            Theme::Dark
        } else {
            Theme::Light
        };
        let background =
            ThemeResources::new(theme, AccentPalette::default()).solid_background_fill_color_base;
        ThemeScope::new(
            theme,
            ColoredBox::new(background).child(
                TabScrollViewer::new(controller, list)
                    .scroll_buttons_visible(app.get(self).show_buttons),
            ),
        )
        .into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    let p = Probe::default();
    let page = p.clone();
    let f = Fixture::with_root([500, 48], move |app| {
        winui_gallery::install_fonts(app);
        let controller = ScrollViewportController::new(app);
        page.controller.set(Some(controller));
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
    (f, p)
}
fn settle(f: &mut Fixture) {
    for _ in 0..3 {
        f.pump();
    }
}

fn buttons(f: &Fixture) -> Vec<(Rect, bool)> {
    let mut output = Vec::new();
    let elements = f.elements();
    let app = f.cell.borrow();
    for element in elements {
        if let Some(button) = downcast_widget::<RepeatButton>(&**element.widget(&app)) {
            let object = element.find_render_object(&app).unwrap().as_box().unwrap();
            let size = object.size(&app);
            let origin = object.local_to_global(&app, Offset::ZERO, None);
            output.push((
                Rect::from_ltwh(origin.dx(), origin.dy(), size.width(), size.height()),
                button.is_enabled,
            ));
        }
    }
    output.sort_by(|a, b| a.0.left.partial_cmp(&b.0.left).unwrap());
    output
}

#[test]
fn source_scroll_buttons_repeat_and_stop_at_edges_in_both_themes() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let (mut f, p) = fixture();
    settle(&mut f);
    let controller = p.controller.get().unwrap();
    let b = buttons(&f);
    assert_eq!(b.len(), 2);
    assert!(!b[0].1 && b[1].1);
    assert_eq!(b[1].0.width(), r.tab_view_item_scroll_button_width);
    assert_eq!(b[1].0.height(), r.tab_view_item_scroll_button_height);
    let point = b[1].0.center();
    f.send_mouse(PointerChange::Add, point, 0);
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    settle(&mut f);
    assert_eq!(controller.metrics(&f.cell.borrow()).offset, 50.0);
    f.cell.elapse(Duration::from_millis(50));
    settle(&mut f);
    assert_eq!(controller.metrics(&f.cell.borrow()).offset, 100.0);
    f.cell.elapse(Duration::from_millis(100));
    settle(&mut f);
    assert_eq!(controller.metrics(&f.cell.borrow()).offset, 150.0);
    f.send_mouse(PointerChange::Up, point, 0);
    f.cell.elapse(Duration::from_millis(500));
    settle(&mut f);
    assert_eq!(controller.metrics(&f.cell.borrow()).offset, 150.0);
    f.capture("tab_scroll_viewer_light");
    p.state
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.dark = true);
    settle(&mut f);
    f.capture("tab_scroll_viewer_dark");
    let maximum = controller.metrics(&f.cell.borrow()).scrollable_length;
    controller.change_view(&mut f.cell.borrow_mut(), maximum, true);
    settle(&mut f);
    let b = buttons(&f);
    assert!(b[0].1 && !b[1].1);
    assert!(p.built.get() < 60, "scrolling template lost virtualization");
}

#[test]
fn hiding_scroll_buttons_preserves_native_list_position_and_changes_viewport_width() {
    let (mut f, p) = fixture();
    settle(&mut f);
    let controller = p.controller.get().unwrap();
    controller.change_view(&mut f.cell.borrow_mut(), 500.0, true);
    settle(&mut f);
    let position = controller
        .native_controller(&f.cell.borrow())
        .position(&f.cell.borrow());
    let width = controller.metrics(&f.cell.borrow()).viewport_length;
    p.state
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.show_buttons = false);
    settle(&mut f);
    assert_eq!(controller.metrics(&f.cell.borrow()).offset, 500.0);
    assert_eq!(
        controller
            .native_controller(&f.cell.borrow())
            .position(&f.cell.borrow()),
        position
    );
    assert!(controller.metrics(&f.cell.borrow()).viewport_length > width);
    assert!(buttons(&f).is_empty());
    f.capture("tab_scroll_viewer_buttons_hidden");
}

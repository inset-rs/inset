//! Compact flyouts retain left presenters and distinguish ancestry from reset indentation.
#![feature(arbitrary_self_types)]

use inset_embedder::{Offset, PointerChange, Rect};
use inset_foundation::{App, Handle};
use inset_rendering::RenderParagraph;
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
    top: bool,
    nav: Rc<GlobalKey>,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> Self::State {
        PageState {
            state: StateData::new(),
            open: true,
            top: false,
            nav: Rc::new(GlobalKey::new()),
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.0.set(Some(self));
        ThemeScope::new(
            Theme::Light,
            NavigationView::new(
                vec![
                    NavigationViewItem::text("root", "Root")
                        .icon(Text::new("P"))
                        .menu_items([
                            NavigationViewItem::header("header", "Nested section"),
                            NavigationViewItem::text("branch", "Branch")
                                .icon(Text::new("C"))
                                .menu_items([NavigationViewItem::text("leaf", "Leaf")]),
                        ]),
                    NavigationViewItem::text("plain", "Text cutoff"),
                ],
                None,
                |_, _| {},
                Text::new("Page"),
            )
            .key(app.get(self).nav.clone())
            .pane_display_mode(if app.get(self).top {
                NavigationViewPaneDisplayMode::Top
            } else {
                NavigationViewPaneDisplayMode::LeftCompact
            })
            .footer_menu_items([NavigationViewItem::text("footer", "Footer")
                .menu_items([NavigationViewItem::text("footer-child", "Footer child")])])
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_settings_visible(false)
            .compact_pane_length(64.0)
            .is_pane_open(app.get(self).open, move |app, open| {
                self.set_state(app, |s| s.open = open)
            }),
        )
        .into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    let p = Probe::default();
    let q = p.clone();
    let mut f = Fixture::with_root([640, 480], move |app| {
        winui_gallery::install_fonts(app);
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
    f.pump();
    f.pump();
    p.0.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.open = true);
    f.cell.checkpoint();
    f.pump();
    f.pump();
    (f, p)
}

fn bounds(f: &Fixture, text: &str) -> Option<Rect> {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().find_map(|e| {
        let mut ancestor = Some(e);
        while let Some(current) = ancestor {
            if downcast_widget::<Offstage>(current.widget(&app).as_ref())
                .is_some_and(|o| o.offstage)
            {
                return None;
            }
            ancestor = current.parent(&app);
        }
        let render = e.render_object(&app)?;
        let paragraph = render.downcast::<RenderParagraph>(&app)?;
        if paragraph.text(&app).to_plain_text(true, true) != text {
            return None;
        }
        let b = render.as_box()?;
        Some(b.local_to_global(&app, Offset::ZERO, None) & b.size(&app))
    })
}

fn item_bounds(f: &Fixture, p: &Probe, id: &str) -> Rect {
    let mut app = f.cell.borrow_mut();
    let key = app.get(p.0.get().unwrap()).nav.clone();
    let nav = key.current_state::<NavigationViewState>(&mut app).unwrap();
    let context = nav.container_from_menu_item(&mut app, id).unwrap();
    let object = context.find_render_object(&app).unwrap().as_box().unwrap();
    object.local_to_global(&app, Offset::ZERO, None) & object.size(&app)
}

fn tap(f: &mut Fixture, label: &str) {
    let at = bounds(f, label).unwrap().center();
    f.send(PointerChange::Down, at);
    f.send(PointerChange::Up, at);
    f.pump();
    f.pump();
}

#[test]
fn compact_flyout_keeps_left_geometry_and_nested_expansion() {
    let (mut f, p) = fixture();
    tap(&mut f, "P");
    let inline_gap = bounds(&f, "Branch").unwrap().left - bounds(&f, "C").unwrap().left;
    tap(&mut f, "Branch");
    assert!(bounds(&f, "Leaf").is_some());
    p.0.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.open = false);
    f.cell.checkpoint();
    f.pump();
    f.pump();
    let root = item_bounds(&f, &p, "root");
    assert!(
        root.width() <= 66.0 && root.right <= 66.0,
        "compact item receives the pane width, not only an outer clip: {root:?}"
    );
    assert!(
        bounds(&f, "Text cutoff").unwrap().width() > 0.0,
        "source compact state cuts off content; it does not hide it"
    );
    f.capture("navigation-compact-64");
    tap(&mut f, "P");
    let branch = bounds(&f, "Branch").expect("compact flyout branch");
    let icon = bounds(&f, "C").unwrap();
    assert!(
        (branch.left - icon.left - inline_gap).abs() < 0.1,
        "left template icon/content geometry must survive flyout reparenting"
    );
    assert!(
        bounds(&f, "Nested section").unwrap().height() > 0.0,
        "depth reset must not make a nested header top-level"
    );
    let leaf = bounds(&f, "Leaf").expect("nested expansion survives top-level force collapse");
    assert!(
        leaf.top > branch.bottom,
        "nested children stay inline in the first flyout"
    );
    f.capture("navigation-compact-left-flyout");
}

#[test]
fn top_footer_children_are_inline_rather_than_primary_flyouts() {
    let (mut f, p) = fixture();
    p.0.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.top = true);
    f.cell.checkpoint();
    f.pump();
    f.pump();
    tap(&mut f, "Footer");
    let elements = f.elements();
    let app = f.cell.borrow();
    let mut found = false;
    for element in elements {
        let Some(paragraph) = element
            .render_object(&app)
            .and_then(|r| r.downcast::<RenderParagraph>(&app))
        else {
            continue;
        };
        if paragraph.text(&app).to_plain_text(true, true) != "Footer child" {
            continue;
        }
        let mut ancestor = Some(element);
        let mut offstage = false;
        let mut portal = false;
        while let Some(current) = ancestor {
            let widget = current.widget(&app);
            offstage |= downcast_widget::<Offstage>(widget.as_ref()).is_some_and(|o| o.offstage);
            portal |= downcast_widget::<OverlayPortal>(widget.as_ref()).is_some();
            ancestor = current.parent(&app);
        }
        if !offstage {
            assert!(
                !portal,
                "TopFooter shares primary visuals but not IsOnTopPrimary's flyout behavior"
            );
            found = true;
        }
    }
    assert!(found, "footer child must be expanded");
    drop(app);
    f.capture("navigation-top-footer-expanded");
}

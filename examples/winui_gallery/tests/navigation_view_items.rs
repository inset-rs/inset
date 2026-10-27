//! NavigationView item templates and hierarchy through native input and GPU layout.
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
    selected: Option<String>,
    events: Vec<String>,
    theme: Theme,
    mode: NavigationViewPaneDisplayMode,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> Self::State {
        PageState {
            state: StateData::new(),
            selected: None,
            events: vec![],
            theme: Theme::Light,
            mode: NavigationViewPaneDisplayMode::Left,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.0.set(Some(self));
        let items = vec![
            NavigationViewItem::header("section", "Section"),
            NavigationViewItem::text("parent", "Parent")
                .icon(FontIcon::symbol(FluentSymbol::Settings))
                .menu_items([NavigationViewItem::text("child", "Child")]),
            NavigationViewItem::separator("separator"),
            NavigationViewItem::text("disabled", "Disabled").is_enabled(false),
            NavigationViewItem::text("action", "Action").selects_on_invoked(false),
        ];
        let mut nav = NavigationView::new(
            items,
            app.get(self).selected.clone(),
            move |app, args| {
                self.set_state(app, |s| {
                    s.selected = args.item.as_ref().map(|item| item.id.clone());
                    s.events.push(format!(
                        "selected:{}",
                        args.item
                            .as_ref()
                            .map(|item| item.id.as_str())
                            .unwrap_or("none")
                    ));
                });
            },
            Text::new("Page content"),
        );
        nav.pane_display_mode = app.get(self).mode;
        nav.is_pane_open = Some(true);
        nav.is_back_button_visible = NavigationViewBackButtonVisible::Collapsed;
        nav.item_invoked = Some(Rc::new(move |app, args| {
            app.get_mut(self)
                .events
                .push(format!("invoked:{}", args.item.id))
        }));
        nav.expanding = Some(Rc::new(move |app, args| {
            app.get_mut(self)
                .events
                .push(format!("expanding:{}", args.item.id))
        }));
        nav.collapsed = Some(Rc::new(move |app, args| {
            app.get_mut(self)
                .events
                .push(format!("collapsed:{}", args.item.id))
        }));
        let theme = app.get(self).theme;
        let background =
            ThemeResources::new(theme, AccentPalette::default()).solid_background_fill_color_base;
        ThemeScope::new(theme, ColoredBox::new(background).child(nav)).into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    let probe = Probe::default();
    let page = probe.clone();
    let f = Fixture::with_root([640, 480], move |app| {
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
        let box_ = render.as_box()?;
        Some(box_.local_to_global(&app, Offset::ZERO, None) & box_.size(&app))
    })
}

#[test]
fn nested_item_indentation_events_disabled_and_action_entries() {
    let (mut f, p) = fixture();
    assert!(bounds(&f, "Section").is_some());
    assert!(bounds(&f, "Child").is_none());
    f.tap("Disabled");
    assert!(f.cell.borrow().get(p.0.get().unwrap()).events.is_empty());
    f.tap("Parent");
    let app = f.cell.borrow();
    assert_eq!(
        app.get(p.0.get().unwrap()).events,
        ["invoked:parent", "selected:parent", "expanding:parent"]
    );
    drop(app);
    let parent = bounds(&f, "Parent").unwrap();
    let child = bounds(&f, "Child").unwrap();
    // Child no-icon content is indented31, while the parent's icon adds32.
    assert!(
        (child.left - parent.left + 1.0).abs() < 1.0,
        "parent={parent:?} child={child:?}"
    );
    f.capture("navigation_items_light_expanded");
    f.tap("Child");
    assert_eq!(
        f.cell.borrow().get(p.0.get().unwrap()).selected.as_deref(),
        Some("child")
    );
    f.tap("Action");
    assert_eq!(
        f.cell.borrow().get(p.0.get().unwrap()).selected.as_deref(),
        Some("child")
    );
    p.0.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.theme = Theme::Dark);
    f.cell.checkpoint();
    f.pump();
    f.capture("navigation_items_dark_expanded");
    f.tap("Parent");
    assert!(bounds(&f, "Child").is_none());
}

#[test]
fn chevron_expands_without_invoking_or_selecting_parent() {
    let (mut f, p) = fixture();
    let label = bounds(&f, "Parent").unwrap();
    let point = Offset::new(292.0, label.center().dy());
    f.send_mouse(PointerChange::Down, point, 1);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
    assert!(bounds(&f, "Child").is_some());
    let app = f.cell.borrow();
    assert_eq!(app.get(p.0.get().unwrap()).events, ["expanding:parent"]);
    assert_eq!(app.get(p.0.get().unwrap()).selected, None);
}

#[test]
fn top_presenters_use_horizontal_geometry_after_left_mode() {
    let (mut f, p) = fixture();
    p.0.get().unwrap().set_state(&mut f.cell.borrow_mut(), |s| {
        s.mode = NavigationViewPaneDisplayMode::Top
    });
    f.cell.checkpoint();
    f.pump();
    f.pump();
    let parent = bounds(&f, "Parent").unwrap();
    let action = bounds(&f, "Action").unwrap();
    assert!(
        action.left > parent.right,
        "parent={parent:?} action={action:?}"
    );
    assert!((action.center().dy() - parent.center().dy()).abs() < 4.0);
    f.capture("navigation_items_top");
}

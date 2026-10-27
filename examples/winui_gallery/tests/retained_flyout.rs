//! Flyout content survives its opener; focus and inherited settings follow the active target.
#![feature(arbitrary_self_types)]

use inset_foundation::{App, Handle, Listener};
use inset_painting::Alignment;
use inset_rendering::MainAxisSize;
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Debug, Default)]
struct Probe {
    page: Rc<Cell<Option<Handle<PageState>>>>,
    content: Rc<Cell<Option<Handle<ContentState>>>>,
    created: Rc<Cell<usize>>,
    disposed: Rc<Cell<usize>>,
    themes: Rc<RefCell<Vec<Theme>>>,
    target_context: Rc<Cell<Option<BuildContext>>>,
}

#[derive(Debug)]
struct Page {
    probe: Probe,
}

struct PageState {
    state: StateData<Page>,
    host: Option<Handle<RetainedFlyoutHost>>,
    first: bool,
    dark: bool,
    opener: Option<AnyFocusNode>,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> Self::State {
        PageState {
            state: StateData::new(),
            host: None,
            first: true,
            dark: false,
            opener: None,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let probe = self.widget(app).probe.clone();
        let host = RetainedFlyoutHost::new(
            app,
            Rc::new(move |_, _| {
                Align::new()
                    .alignment(Alignment::BOTTOM_CENTER.into())
                    .child(Content {
                        probe: probe.clone(),
                    })
                    .into_widget()
            }),
        );
        app.get_mut(self).host = Some(host);
        app.get_mut(self).opener = Some(FocusNode::new(app).as_node());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).host.unwrap().dispose(app);
        let opener = app.get(self).opener.unwrap();
        opener.dispose(app);
        app.destroy(opener.id());
    }

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).probe.page.set(Some(self));
        let host = app.get(self).host.unwrap();
        let opener = app.get(self).opener.unwrap();
        let probe = self.widget(app).probe.clone();
        let target = |name: &'static str| {
            let probe = probe.clone();
            FlyoutTarget::new(Builder::new(move |_, context| {
                probe.target_context.set(Some(context));
                Button::text(
                    name,
                    Listener::new(move |app| host.show_at(app, context, true)),
                )
                .focus_node(opener)
                .into_widget()
            }))
            .key(Rc::new(inset_foundation::ValueKey::new(name)))
            .into_widget()
        };
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
            .child(Align::new().alignment(Alignment::TOP_LEFT.into()).child(
                Column::new().main_axis_size(MainAxisSize::Min).children([
                    if app.get(self).first {
                        target("First target")
                    } else {
                        target("Second target")
                    },
                    Button::text("Hide", Listener::new(move |app| host.hide(app))).into_widget(),
                ]),
            )),
        )
        .into_widget()
    }
}

#[derive(Debug)]
struct Content {
    probe: Probe,
}

struct ContentState {
    state: StateData<Content>,
    count: usize,
    focus: Option<AnyFocusNode>,
}

impl StatefulWidget for Content {
    type State = ContentState;

    fn create_state(&self) -> Self::State {
        self.probe.created.set(self.probe.created.get() + 1);
        ContentState {
            state: StateData::new(),
            count: 0,
            focus: None,
        }
    }
}

impl State for ContentState {
    type Widget = Content;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).focus = Some(FocusNode::new(app).as_node());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let probe = self.widget(app).probe.clone();
        probe.disposed.set(probe.disposed.get() + 1);
        let focus = app.get(self).focus.unwrap();
        focus.dispose(app);
        app.destroy(focus.id());
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let probe = self.widget(app).probe.clone();
        probe.content.set(Some(self));
        let theme = ThemeResources::of(app, context).theme;
        probe.themes.borrow_mut().push(theme);
        Button::text(
            format!("Child count: {}", app.get(self).count),
            Listener::new(move |app| {
                self.set_state(app, |state| state.count += 1);
            }),
        )
        .focus_node(app.get(self).focus.unwrap())
        .into_widget()
    }
}

fn fixture() -> (Fixture, Probe) {
    let probe = Probe::default();
    let shared = probe.clone();
    let fixture = Fixture::with_root([400, 320], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                Page {
                    probe: shared.clone(),
                }
                .into_widget()
            }),
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
    (fixture, probe)
}

#[test]
fn child_state_survives_closing_and_removing_its_original_target() {
    let (mut f, probe) = fixture();
    assert_eq!(probe.created.get(), 0);
    f.tap("First target");
    f.tap("Child count: 0");
    f.find("Child count: 1");
    let original = probe.content.get().unwrap();
    f.tap("Hide");
    probe
        .page
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |state| state.first = false);
    f.pump();
    assert_eq!(probe.disposed.get(), 0);
    f.tap("Second target");
    f.find("Child count: 1");
    assert_eq!(probe.content.get(), Some(original));
    assert_eq!(probe.created.get(), 1);
    let host = f.cell.borrow().get(probe.page.get().unwrap()).host.unwrap();
    host.dispose(&mut f.cell.borrow_mut());
    f.pump();
    assert_eq!(probe.disposed.get(), 1);
}

#[test]
fn removing_an_open_target_hides_without_disposing_content() {
    let (mut f, probe) = fixture();
    f.tap("First target");
    let page = probe.page.get().unwrap();
    page.set_state(&mut f.cell.borrow_mut(), |state| state.first = false);
    f.pump();
    let app = f.cell.borrow();
    let host = app.get(page).host.unwrap();
    assert!(!host.is_open(&app));
    drop(app);
    assert_eq!(probe.disposed.get(), 0);
    f.tap("Second target");
    f.find("Child count: 0");
    assert_eq!(probe.created.get(), 1);
}

#[test]
fn focus_returns_to_opener_and_target_theme_updates_reach_retained_content() {
    let (mut f, probe) = fixture();
    let page = probe.page.get().unwrap();
    let opener = f.cell.borrow().get(page).opener.unwrap();
    opener.request_focus(&mut f.cell.borrow_mut(), None);
    f.pump();
    f.tap("First target");
    let content = probe.content.get().unwrap();
    let expected_focus = f.cell.borrow().get(content).focus;
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), expected_focus);
    page.set_state(&mut f.cell.borrow_mut(), |state| state.dark = true);
    f.pump();
    assert_eq!(probe.themes.borrow().last(), Some(&Theme::Dark));
    f.capture("retained_flyout_dark");
    f.tap("Hide");
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), Some(opener));
    f.capture("retained_flyout_hidden");
}

#[test]
fn a_non_focus_taking_open_preserves_current_focus() {
    let (mut f, probe) = fixture();
    let page = probe.page.get().unwrap();
    let opener = f.cell.borrow().get(page).opener.unwrap();
    let host = f.cell.borrow().get(page).host.unwrap();
    opener.request_focus(&mut f.cell.borrow_mut(), None);
    f.pump();
    host.show_at(
        &mut f.cell.borrow_mut(),
        probe.target_context.get().unwrap(),
        false,
    );
    f.pump();
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), Some(opener));
    host.hide(&mut f.cell.borrow_mut());
    f.pump();
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), Some(opener));
}

#[test]
fn root_teardown_disposes_retained_content_once() {
    let (mut f, probe) = fixture();
    f.tap("First target");
    let host = f.cell.borrow().get(probe.page.get().unwrap()).host.unwrap();
    run_app(&mut f.cell.borrow_mut(), SizedBox::shrink().into_widget());
    f.cell.elapse(std::time::Duration::ZERO);
    f.pump();
    assert_eq!(probe.disposed.get(), 1);
    assert!(!host.is_open(&f.cell.borrow()));
    host.dispose(&mut f.cell.borrow_mut());
    assert_eq!(probe.disposed.get(), 1);
}

#[test]
fn disposing_before_the_first_frame_cancels_content_creation_and_focus_work() {
    let (mut f, probe) = fixture();
    let page = probe.page.get().unwrap();
    let host = f.cell.borrow().get(page).host.unwrap();
    let opener = f.cell.borrow().get(page).opener.unwrap();
    opener.request_focus(&mut f.cell.borrow_mut(), None);
    f.pump();
    host.show_at(
        &mut f.cell.borrow_mut(),
        probe.target_context.get().unwrap(),
        true,
    );
    host.dispose(&mut f.cell.borrow_mut());
    f.pump();
    assert_eq!(probe.created.get(), 0);
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), Some(opener));
}

//! The gallery's navigation, driven through the real pages.
//!
//! These tests boot the same app `main.rs` boots, so a page that panics while building, a
//! catalog arm that names a missing sub-page, or a route that stops carrying its title all fail
//! here.
#![feature(arbitrary_self_types)]

use std::rc::Rc;
use std::time::Duration;

use cupertino_gallery::{Entry, entry_route, gallery, sub_route};
use reveal_cupertino::{CupertinoNavigationBarBackButton, install_cupertino_icon_font};
use reveal_embedder::{
    Offset, Picture, Platform, PlatformRef, PointerChange, PointerData, PointerDataPacket,
    PointerDeviceKind, TargetPlatform, View as EmbedderView, ViewConstraints, ViewId, ViewMetrics,
    ViewRef,
};
use reveal_foundation::AppCell;
use reveal_gestures::GestureBinding;
use reveal_painting::PaintingBinding;
use reveal_rendering::{AnyRenderObject, RenderParagraph};
use reveal_scheduler::SchedulerBinding;
use reveal_widgets::{
    AnyElement, AnyRoute, GlobalKey, IntoWidget, NavigatorState, Text, WidgetsBinding,
    downcast_widget, run_app,
};

/// The window `main.rs` opens, in logical points.
const VIEW: [f64; 2] = [420.0, 720.0];

/// A 420x720 view at 1x that presents nowhere.
struct TestView;

impl EmbedderView for TestView {
    fn id(&self) -> ViewId {
        ViewId(0)
    }

    fn metrics(&self) -> ViewMetrics {
        ViewMetrics {
            physical_size: VIEW,
            physical_constraints: ViewConstraints::tight(VIEW[0], VIEW[1]),
            device_pixel_ratio: 1.0,
            ..ViewMetrics::default()
        }
    }

    fn present(&self, _picture: &Picture) {}
}

/// A platform whose implicit view is [`TestView`], so `run_app` finds one to attach to.
struct TestPlatform {
    view: ViewRef,
}

impl Platform for TestPlatform {
    fn target_platform(&self) -> TargetPlatform {
        TargetPlatform::IOS
    }

    fn request_frame(&self) {}

    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }

    fn wake_at(&self, _deadline: std::time::Instant) {}

    fn views(&self) -> Vec<ViewRef> {
        vec![Rc::clone(&self.view)]
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        (self.view.id() == id).then(|| Rc::clone(&self.view))
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        Some(Rc::clone(&self.view))
    }
}

/// Which of several paragraphs reading the same text a tap should aim at.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Search {
    /// The one an actual tap would land on.
    Topmost,
    /// The first in tree order, for a screen that repeats a label.
    First,
}

/// The mounted gallery, plus the key its navigator is reached through.
struct Fixture {
    cell: Rc<AppCell>,
    navigator: GlobalKey,
    at: Duration,
}

impl Fixture {
    fn new() -> Fixture {
        let platform: PlatformRef = Rc::new(TestPlatform {
            view: Rc::new(TestView),
        });
        let cell = AppCell::with_platform(platform);
        let mut app = cell.borrow_mut();
        // What the shell does at start-up: the app-wide fonts every paragraph shapes against.
        PaintingBinding::instance(&mut app).install_fonts(&mut app, |fonts| {
            fonts.add_source(valo_system_fonts::SystemFonts::load());
        });

        let navigator = GlobalKey::new();
        run_app(
            &mut app,
            gallery().navigator_key(navigator.clone()).into_widget(),
        );
        // The same ordering constraint the binary has: `run_app` is what attaches the tree the
        // icon glyphs are shaped in.
        install_cupertino_icon_font(&mut app);
        // `run_app` attaches the root widget on the next timer turn, as Dart's `Timer.run` does.
        drop(app);
        cell.elapse(Duration::ZERO);

        let mut fixture = Fixture {
            cell,
            navigator,
            at: Duration::ZERO,
        };
        fixture.settle();
        fixture
    }

    /// Pumps a fixed span of frames rather than waiting for quiescence: the Activity entry's
    /// spinner never stops asking for the next frame, so "settled" is not a state this app
    /// reaches. Forty frames is 800 ms — comfortably past the route transition.
    fn settle(&mut self) {
        for _ in 0..40 {
            self.at += Duration::from_millis(20);
            SchedulerBinding::handle_begin_frame(&mut self.cell.borrow_mut(), Some(self.at));
            self.cell.borrow_mut().drain_microtasks();
            SchedulerBinding::handle_draw_frame(&mut self.cell.borrow_mut());
            self.cell.borrow_mut().drain_microtasks();
        }
    }

    fn navigator(&mut self) -> reveal_foundation::Handle<NavigatorState> {
        self.navigator
            .current_state::<NavigatorState>(&mut self.cell.borrow_mut())
            .expect("the CupertinoApp's navigator is mounted")
    }

    fn push(&mut self, route: AnyRoute) {
        let navigator = self.navigator();
        navigator.push(&mut self.cell.borrow_mut(), route);
        self.settle();
    }

    /// The same call the row's chevron makes.
    fn push_entry(&mut self, entry: Entry) {
        let route = entry_route(&mut self.cell.borrow_mut(), entry);
        self.push(route);
    }

    fn push_sub_page(&mut self, entry: Entry, index: usize) {
        let route = sub_route(&mut self.cell.borrow_mut(), entry, index);
        self.push(route);
    }

    fn pop(&mut self) {
        let navigator = self.navigator();
        navigator.pop(&mut self.cell.borrow_mut(), None);
        self.settle();
    }

    fn can_pop(&mut self) -> bool {
        let navigator = self.navigator();
        navigator.can_pop(&self.cell.borrow())
    }

    /// Every render object of the mounted tree, parents before children.
    fn render_objects(&mut self) -> Vec<AnyRenderObject> {
        let root = {
            let mut app = self.cell.borrow_mut();
            WidgetsBinding::instance(&mut app)
                .root_element(&app)
                .expect("a mounted app")
                .find_render_object(&app)
                .expect("a mounted view has a render object")
        };
        let mut all = vec![root];
        let mut visited = 0;
        while visited < all.len() {
            let object = all[visited];
            object.visit_children(&self.cell.borrow(), &mut |child| all.push(child));
            visited += 1;
        }
        all
    }

    /// The mounted elements in tree order: a subtree is visited before its next sibling, so the
    /// overlay's topmost route comes last.
    fn elements(&mut self) -> Vec<AnyElement> {
        let root = {
            let mut app = self.cell.borrow_mut();
            WidgetsBinding::instance(&mut app)
                .root_element(&app)
                .expect("a mounted app")
        };
        let mut all = Vec::new();
        let mut stack = vec![root];
        while let Some(element) = stack.pop() {
            all.push(element);
            let mut children = element.children(&self.cell.borrow());
            children.reverse();
            stack.extend(children);
        }
        all
    }

    /// What the top screen's automatic back button says.
    ///
    /// Every route in the stack contributes a bar, so this takes the LAST back button in tree
    /// order — the overlay paints its entries in order, so that one belongs to the route on top.
    fn back_label(&mut self) -> String {
        let elements = self.elements();
        let button = elements
            .iter()
            .rev()
            .find(|element| {
                downcast_widget::<CupertinoNavigationBarBackButton>(
                    &**element.widget(&self.cell.borrow()),
                )
                .is_some()
            })
            .copied()
            .expect("the top screen has an automatic back button");

        let mut labels = Vec::new();
        let mut stack = vec![button];
        while let Some(element) = stack.pop() {
            if let Some(text) = downcast_widget::<Text>(&**element.widget(&self.cell.borrow()))
                && let Some(data) = &text.data
            {
                labels.push(data.clone());
            }
            stack.extend(element.children(&self.cell.borrow()));
        }
        labels.join(" ")
    }

    /// The view-space centre of a paragraph containing `text`.
    ///
    /// [`Search::Topmost`] is the default a tap wants: every route in the stack keeps its
    /// elements, so the index's own rows are still there under a pushed screen, and the overlay
    /// paints its last entry on top. [`Search::First`] is for a screen that shows one label
    /// several times over.
    fn center_of(&mut self, text: &str, order: Search) -> Offset {
        let mut elements = self.elements();
        if order == Search::Topmost {
            elements.reverse();
        }
        for element in elements {
            let Some(object) = element.render_object(&self.cell.borrow()) else {
                continue;
            };
            let Some(paragraph) = object.downcast::<RenderParagraph>(&self.cell.borrow()) else {
                continue;
            };
            if !paragraph
                .text(&self.cell.borrow())
                .to_plain_text(true, true)
                .contains(text)
            {
                continue;
            }
            let object = object.as_box().expect("a paragraph is a box");
            let size = object.size(&self.cell.borrow());
            let origin = object.local_to_global(&self.cell.borrow(), Offset::ZERO, None);
            return origin + Offset::new(size.width() / 2.0, size.height() / 2.0);
        }
        panic!("nothing on screen reads {text:?}; saw {:?}", self.texts())
    }

    /// Taps the middle of the topmost thing showing `text`.
    fn tap(&mut self, text: &str) {
        let point = self.center_of(text, Search::Topmost);
        self.tap_at(point);
    }

    /// Taps the middle of the first thing showing `text`, in tree order.
    fn tap_first(&mut self, text: &str) {
        let point = self.center_of(text, Search::First);
        self.tap_at(point);
    }

    fn tap_at(&mut self, point: Offset) {
        self.send(PointerChange::Down, point);
        self.at += Duration::from_millis(20);
        self.send(PointerChange::Up, point);
        self.settle();
    }

    fn send(&mut self, change: PointerChange, at_point: Offset) {
        {
            let mut app = self.cell.borrow_mut();
            GestureBinding::instance(&mut app).handle_pointer_data_packet(
                &mut app,
                PointerDataPacket::new(vec![PointerData {
                    change,
                    kind: PointerDeviceKind::Touch,
                    time_stamp: self.at,
                    pointer_identifier: 1,
                    physical_x: at_point.dx(),
                    physical_y: at_point.dy(),
                    ..PointerData::default()
                }]),
            );
        }
        self.cell.borrow_mut().drain_microtasks();
    }

    /// The plain text of every paragraph on screen.
    fn texts(&mut self) -> Vec<String> {
        self.render_objects()
            .into_iter()
            .filter_map(|object| object.downcast::<RenderParagraph>(&self.cell.borrow()))
            .map(|paragraph| {
                paragraph
                    .text(&self.cell.borrow())
                    .to_plain_text(true, true)
            })
            .collect()
    }

    fn shows(&mut self, text: &str) -> bool {
        self.texts().iter().any(|shown| shown.contains(text))
    }
}

/// A line each entry's screen shows and no other screen does — near the top of the page, since
/// a `ListView` only builds what is close to the viewport.
fn marker(entry: Entry) -> &'static str {
    match entry {
        Entry::Navigation => "Standard bar, short title",
        Entry::PageScaffold => "Body starts here",
        Entry::Buttons => "Custom colour",
        Entry::Segments => "Selected: Midnight",
        Entry::Dialogs => "Ask a question",
        Entry::Scrolling => "Rows 1\u{2013}10",
        Entry::Icons => "house_fill",
        Entry::Text => "action_small_text_style",
        Entry::Heroes => "Swatches",
        Entry::Indicators => "Partially revealed",
        Entry::Theming => "SECONDARY_LABEL",
        Entry::Lists => "Title only",
        Entry::ExpansionTiles => "Fade transition",
    }
}

/// A line each sub-screen shows: its own title, which the route carries into the bar.
fn sub_marker(entry: Entry, index: usize) -> &'static str {
    entry.sub_title(index)
}

#[test]
fn the_index_lists_every_entry() {
    let mut fixture = Fixture::new();
    let texts = fixture.texts();
    for entry in Entry::ALL {
        assert!(
            texts.iter().any(|text| text == entry.title()),
            "the index has no row for {entry:?}; saw {texts:?}"
        );
    }
    assert!(
        texts.iter().any(|text| text == "Gallery"),
        "the index wears its own title; saw {texts:?}"
    );
}

#[test]
fn every_entry_builds_and_pops() {
    let mut fixture = Fixture::new();
    for entry in Entry::ALL {
        fixture.push_entry(entry);
        assert!(
            fixture.shows(marker(entry)),
            "{entry:?} did not show {:?}; saw {:?}",
            marker(entry),
            fixture.texts()
        );
        fixture.pop();
        assert!(
            !fixture.shows(marker(entry)),
            "{entry:?} is still on screen after popping"
        );
        assert!(
            !fixture.can_pop(),
            "the index is back on top after {entry:?}"
        );
    }
}

#[test]
fn every_sub_page_builds() {
    let mut fixture = Fixture::new();
    for entry in Entry::ALL {
        for index in 0..entry.sub_page_count() {
            fixture.push_entry(entry);
            fixture.push_sub_page(entry, index);
            assert!(
                fixture.shows(sub_marker(entry, index)),
                "{entry:?} sub-page {index} did not show {:?}; saw {:?}",
                sub_marker(entry, index),
                fixture.texts()
            );
            fixture.pop();
            fixture.pop();
        }
    }
    assert!(!fixture.can_pop(), "every sub-page came back off");
}

/// The route's title is what the bar's automatic middle shows and what the next screen's back
/// button reads — the whole reason the index is generated rather than handed over as `home`.
#[test]
fn a_pushed_screen_wears_its_title_and_the_index_as_its_back_label() {
    let mut fixture = Fixture::new();
    fixture.push_entry(Entry::Buttons);
    assert!(
        fixture.shows(Entry::Buttons.title()),
        "the bar's middle is the route's title; saw {:?}",
        fixture.texts()
    );
    assert!(
        fixture.shows("Gallery"),
        "the back button reads the route below; saw {:?}",
        fixture.texts()
    );
}

/// The back label keeps a previous title verbatim only up to twelve characters, which is what
/// the Navigation entry's chain of pushes is built to show.
#[test]
fn a_long_previous_title_collapses_the_back_label() {
    let mut fixture = Fixture::new();
    fixture.push_entry(Entry::Navigation);
    fixture.push_sub_page(Entry::Navigation, 0);
    assert_eq!(
        fixture.back_label(),
        "Navigation",
        "ten characters, kept verbatim"
    );

    fixture.push_sub_page(Entry::Navigation, 1);
    assert_eq!(
        fixture.back_label(),
        "Second Item",
        "eleven characters, still kept verbatim"
    );

    fixture.push_sub_page(Entry::Navigation, 2);
    assert_eq!(
        fixture.back_label(),
        "Back",
        "twenty-three characters: the label falls back to the localized \"Back\""
    );
}

/// The index's rows really push: nothing in the tests above went through `on_tap`.
#[test]
fn tapping_a_row_opens_its_entry() {
    let mut fixture = Fixture::new();
    fixture.tap(Entry::Buttons.title());
    assert!(
        fixture.shows(marker(Entry::Buttons)),
        "the row pushed the Buttons screen; saw {:?}",
        fixture.texts()
    );
    assert!(fixture.can_pop(), "the index is still underneath");
}

/// The dialogs page really shows a dialog, and the answer really comes back when it pops.
#[test]
fn an_alert_dialog_opens_and_reports_its_answer() {
    let mut fixture = Fixture::new();
    fixture.push_entry(Entry::Dialogs);
    assert!(
        fixture.shows("\u{2014}"),
        "no answer yet; saw {:?}",
        fixture.texts()
    );

    fixture.tap("Ask a question");
    assert!(
        fixture.shows("The thing will be done."),
        "the alert dialog is up; saw {:?}",
        fixture.texts()
    );

    fixture.tap("OK");
    assert!(
        !fixture.shows("The thing will be done."),
        "the action popped the dialog; saw {:?}",
        fixture.texts()
    );
    assert!(
        fixture.shows("Yes"),
        "the popped result reached the page; saw {:?}",
        fixture.texts()
    );
}

/// The segmented control reports the tap; the page decides what it means.
#[test]
fn tapping_a_segment_moves_the_value_it_is_bound_to() {
    let mut fixture = Fixture::new();
    fixture.push_entry(Entry::Segments);
    assert!(
        fixture.shows("Selected: Midnight"),
        "the preview starts on the initial group_value; saw {:?}",
        fixture.texts()
    );

    // The first of the three controls on the page. The third keeps its own selection, so a tap
    // on that one would leave the preview where it is — which is the point of the third section.
    fixture.tap_first("Cerulean");
    assert!(
        fixture.shows("Selected: Cerulean"),
        "on_value_changed reached the page state; saw {:?}",
        fixture.texts()
    );

    fixture.tap("Viridian");
    assert!(
        fixture.shows("Selected: Cerulean"),
        "the last control keeps its own value; saw {:?}",
        fixture.texts()
    );
}

/// A header tap and a button reach the same expansion, through the one controller they share.
#[test]
fn an_expansion_tile_opens_from_its_header_and_from_its_controller() {
    let mut fixture = Fixture::new();
    fixture.push_entry(Entry::ExpansionTiles);
    assert!(
        fixture.shows("is_expanded: false"),
        "the controlled tile starts closed; saw {:?}",
        fixture.texts()
    );

    fixture.tap("Controlled");
    assert!(
        fixture.shows("is_expanded: true"),
        "the header tap drove the shared ExpansibleController; saw {:?}",
        fixture.texts()
    );

    // The button's own label followed the expansion, which is how the page reads the controller.
    fixture.tap("Collapse");
    assert!(
        fixture.shows("is_expanded: false"),
        "the button drove the same controller back; saw {:?}",
        fixture.texts()
    );
}

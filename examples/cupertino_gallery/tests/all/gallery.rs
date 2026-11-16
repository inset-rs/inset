//! The gallery's navigation, driven through the real pages.
//!
//! These tests boot the same app `main.rs` boots, so a page that panics while building, a
//! catalog arm that names a missing sub-page, or a route that stops carrying its title all fail
//! here.

use std::time::Duration;

use cupertino_gallery_lib::{Entry, entry_route, gallery, sub_route};
use waft_cupertino::CupertinoNavigationBarBackButton;
use waft_embedder::{Offset, Size, TargetPlatform};
use waft_rendering::RenderParagraph;
use waft_test::TestPlatform;
use waft_widgets::testing::WidgetTester;
use waft_widgets::{AnyElement, GlobalKey, NavigatorState, RouteRef, Text, downcast_widget};

/// The window `main.rs` opens, in logical points.
const VIEW: [f64; 2] = [420.0, 720.0];

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
    tester: WidgetTester,
    navigator: GlobalKey,
}

impl Fixture {
    fn new() -> Fixture {
        // The window at flutter_test's device pixel ratio, on the platform the gallery is for.
        let mut tester = WidgetTester::with_physical_size(
            TestPlatform::new().on(TargetPlatform::IOS),
            Size::new(
                VIEW[0] * WidgetTester::DEVICE_PIXEL_RATIO,
                VIEW[1] * WidgetTester::DEVICE_PIXEL_RATIO,
            ),
        );
        let navigator = GlobalKey::new();
        tester.pump_widget(gallery().navigator_key(navigator.clone()));

        let mut fixture = Fixture { tester, navigator };
        fixture.settle();
        fixture
    }

    /// Pumps a fixed span of frames rather than waiting for quiescence: the Activity entry's
    /// spinner never stops asking for the next frame, so "settled" is not a state this app
    /// reaches. Forty frames is 800 ms — comfortably past the route transition.
    fn settle(&mut self) {
        for _ in 0..40 {
            self.tester.pump_by(Duration::from_millis(20));
        }
    }

    /// One frame, rather than the forty `settle` pumps, for watching a transition start.
    fn frame(&mut self) {
        self.tester.pump_by(Duration::from_millis(16));
    }

    /// Where the topmost thing reading `text` sits, or `None` when nothing does.
    fn position_of(&mut self, text: &str) -> Option<Offset> {
        let mut elements = self.elements();
        elements.reverse();
        for element in elements {
            let Some(object) = element.render_object(&self.tester.cell.borrow()) else {
                continue;
            };
            let Some(paragraph) = object.downcast::<RenderParagraph>(&self.tester.cell.borrow())
            else {
                continue;
            };
            if !paragraph
                .text(&self.tester.cell.borrow())
                .to_plain_text(true, true)
                .contains(text)
            {
                continue;
            }
            let object = object.as_box().expect("a paragraph is a box");
            return Some(object.local_to_global(&self.tester.cell.borrow(), Offset::ZERO, None));
        }
        None
    }

    fn navigator(&mut self) -> waft_foundation::Handle<NavigatorState> {
        self.navigator
            .current_state::<NavigatorState>(&self.tester.cell.borrow_mut())
            .expect("the CupertinoApp's navigator is mounted")
    }

    fn push(&mut self, route: RouteRef) {
        let navigator = self.navigator();
        navigator.push(&mut self.tester.cell.borrow_mut(), route);
        self.settle();
    }

    /// The same call the row's chevron makes.
    fn push_entry(&mut self, entry: Entry) {
        let route = entry_route(&mut self.tester.cell.borrow_mut(), entry);
        self.push(route);
    }

    fn push_sub_page(&mut self, entry: Entry, index: usize) {
        let route = sub_route(&mut self.tester.cell.borrow_mut(), entry, index);
        self.push(route);
    }

    fn pop(&mut self) {
        let navigator = self.navigator();
        navigator.pop(&mut self.tester.cell.borrow_mut(), None);
        self.settle();
    }

    fn can_pop(&mut self) -> bool {
        let navigator = self.navigator();
        navigator.can_pop(&self.tester.cell.borrow())
    }

    /// The mounted elements in tree order: a subtree is visited before its next sibling, so the
    /// overlay's topmost route comes last.
    fn elements(&mut self) -> Vec<AnyElement> {
        self.tester.elements()
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
                    &**element.widget(&self.tester.cell.borrow()),
                )
                .is_some()
            })
            .copied()
            .expect("the top screen has an automatic back button");

        let mut labels = Vec::new();
        let mut stack = vec![button];
        while let Some(element) = stack.pop() {
            if let Some(text) =
                downcast_widget::<Text>(&**element.widget(&self.tester.cell.borrow()))
                && let Some(data) = &text.data
            {
                labels.push(data.clone());
            }
            stack.extend(element.children(&self.tester.cell.borrow()));
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
            let Some(object) = element.render_object(&self.tester.cell.borrow()) else {
                continue;
            };
            let Some(paragraph) = object.downcast::<RenderParagraph>(&self.tester.cell.borrow())
            else {
                continue;
            };
            if !paragraph
                .text(&self.tester.cell.borrow())
                .to_plain_text(true, true)
                .contains(text)
            {
                continue;
            }
            let object = object.as_box().expect("a paragraph is a box");
            let size = object.size(&self.tester.cell.borrow());
            let origin = object.local_to_global(&self.tester.cell.borrow(), Offset::ZERO, None);
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

    /// Drags the screen up by `distance` from the middle of the window, as a finger scrolls
    /// a page to what is below the fold, and lets the scroll come to rest.
    fn scroll_up(&mut self, distance: f64) {
        let middle = Offset::new(VIEW[0] / 2.0, VIEW[1] / 2.0);
        self.tester
            .drag_from(middle, Offset::new(0.0, -distance), 1);
        self.settle();
    }

    fn tap_at(&mut self, point: Offset) {
        self.tester.tap_at(point, 1);
        self.settle();
    }

    /// The plain text of every paragraph on screen.
    fn texts(&mut self) -> Vec<String> {
        let paragraphs = self.tester.all::<RenderParagraph>();
        let app = self.tester.cell.borrow();
        paragraphs
            .into_iter()
            .map(|paragraph| paragraph.text(&app).to_plain_text(true, true))
            .collect()
    }

    fn shows(&mut self, text: &str) -> bool {
        self.texts().iter().any(|shown| shown.contains(text))
    }

    /// Whether a paragraph on screen reads exactly `text`, for a value that other text on the
    /// page may contain, such as a dash in a footer.
    fn shows_paragraph(&mut self, text: &str) -> bool {
        self.texts().iter().any(|shown| shown == text)
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
        Entry::Switches => "Airplane Mode off, Wi-Fi on",
        Entry::Dialogs => "Ask a question",
        Entry::Scrolling => "Rows 1\u{2013}10",
        Entry::Icons => "house_fill",
        Entry::Images => "Animated GIF",
        Entry::Text => "action_small_text_style",
        Entry::TextFields => "Type here",
        Entry::Heroes => "Swatches",
        Entry::Indicators => "Partially revealed",
        Entry::Theming => "SECONDARY_LABEL",
        Entry::Lists => "Title only",
        Entry::ExpansionTiles => "Fade transition",
        Entry::ShoppingCart => "Espresso",
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
    let button = fixture.position_of("Ask a question");
    // FlutterTest's square glyphs wrap the footers onto more lines than the system fonts do,
    // which puts the Result row below the fold, where the list does not build it.
    fixture.scroll_up(VIEW[1]);
    assert!(
        fixture.shows("Last answer") && fixture.shows_paragraph("\u{2014}"),
        "no answer yet; saw {:?}",
        fixture.texts()
    );
    // Back to the top, where the buttons are, in drags that stay inside the window.
    for _ in 0..10 {
        if fixture.position_of("Ask a question") == button {
            break;
        }
        fixture.scroll_up(-VIEW[1] / 2.0);
    }

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
    fixture.scroll_up(VIEW[1]);
    assert!(
        fixture.shows("Last answer") && fixture.shows_paragraph("Yes"),
        "the popped result reached the Result row; saw {:?}",
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

/// A switch reports the flip; the page decides what it means, so the tile around it flips it too.
#[test]
fn tapping_a_settings_row_flips_the_switch_it_holds() {
    let mut fixture = Fixture::new();
    fixture.push_entry(Entry::Switches);
    assert!(
        fixture.shows("Airplane Mode off, Wi-Fi on"),
        "the summary starts on the initial values; saw {:?}",
        fixture.texts()
    );

    // The tile's title comes before the summary that repeats it.
    fixture.tap_first("Airplane Mode");
    assert!(
        fixture.shows("Airplane Mode on, Wi-Fi on"),
        "the tile's tap reached the page state; saw {:?}",
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

/// A tap updates the cart Entity; the summary rebuilds, and the cart route reads the same store.
#[test]
fn a_cart_tap_rebuilds_from_the_entity_and_the_cart_route_sees_it() {
    let mut fixture = Fixture::new();
    fixture.push_entry(Entry::ShoppingCart);
    assert!(
        fixture.shows("Empty"),
        "the cart starts empty; saw {:?}",
        fixture.texts()
    );

    fixture.tap("Espresso");
    assert!(
        fixture.shows("1 item"),
        "the cart row read Totals after Cart notified; saw {:?}",
        fixture.texts()
    );
    assert!(
        fixture.shows("$4.00"),
        "the menu price and the cart total stay in dollars; saw {:?}",
        fixture.texts()
    );

    fixture.tap("Your cart");
    assert!(
        fixture.shows("Espresso"),
        "the cart route shares the Entity; saw {:?}",
        fixture.texts()
    );
}

/// A push measures the route ahead by putting it offstage, which reads its animation as
/// complete. `_updateSecondaryAnimation` follows the next route's own animation rather than
/// that reading, so the page behind must sit still on that frame instead of jumping to where
/// the transition ends and starting over.
#[test]
fn the_page_behind_sits_still_on_the_frame_a_push_is_measured() {
    let mut fixture = Fixture::new();
    let resting = fixture.position_of("Widgets").expect("the index is up");

    let route = entry_route(&mut fixture.tester.cell.borrow_mut(), Entry::Segments);
    let navigator = fixture.navigator();
    navigator.push(&mut fixture.tester.cell.borrow_mut(), route);

    fixture.frame();
    assert_eq!(
        fixture.position_of("Widgets"),
        Some(resting),
        "the index moved on the frame the pushed route was measured"
    );

    fixture.frame();
    let sliding = fixture
        .position_of("Widgets")
        .expect("the index is still up");
    assert!(
        sliding.dx() < resting.dx(),
        "the index should slide left once the transition runs, from {resting:?} to {sliding:?}"
    );
}

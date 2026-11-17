//! The gallery's navigation, driven through the real pages.
//!
//! These tests boot the same app `main.rs` boots, so a page that panics while building, a
//! catalog arm that names a missing sub-page, or a page whose app bar loses its title or its
//! back button all fail here. Every case runs on Android, the platform Flutter's own tests
//! run on, and then on macOS, where `MaterialPageRoute` slides as Cupertino's does and the
//! scroll views wear desktop scrollbars.

use std::time::Duration;

use material_gallery_lib::pages::cards::EXPANDED_TILE_CHILD;
use material_gallery_lib::pages::drawers::{DRAWER_HEADER, END_DRAWER_HEADER};
use material_gallery_lib::pages::home::INDEX_TITLE;
use material_gallery_lib::pages::messages::{BANNER_TEXT, PERSISTENT_SHEET_TEXT};
use material_gallery_lib::pages::steppers::LAST_ACTION;
use material_gallery_lib::{Entry, Gallery, entry_route, sub_route};
use waft_embedder::{Brightness, Offset, Size, TargetPlatform};
use waft_material::{
    AppBar, BackButton, Drawer, DrawerButton, DrawerHeader, ElevatedButton, EndDrawerButton,
    FilledButton, IconButton, ListTile, NavigationRail, OutlinedButton, Scaffold, SliverAppBar,
    SnackBar, TextButton, Theme,
};
use waft_rendering::{RenderAnimatedOpacity, RenderAnimatedOpacityMixin, RenderOpacity};
use waft_widgets::testing::{
    Finder, TargetPlatformVariant, WidgetTester, expect, find, finds_nothing, finds_one_widget,
    finds_widgets, test_widgets_variant,
};
use waft_widgets::{GlobalKey, NavigatorState, RouteRef, Text, WidgetsBinding, Wrap};

/// The window `main.rs` opens, in logical points.
const VIEW: [f64; 2] = [420.0, 720.0];

/// A phone, and a desktop.
const PLATFORMS: [TargetPlatform; 2] = [TargetPlatform::Android, TargetPlatform::MacOS];

/// Runs `case` against a fresh gallery on each of [`PLATFORMS`], naming the platform it
/// failed on.
fn on_each_platform(case: fn(&mut Fixture)) {
    test_widgets_variant(TargetPlatformVariant::new(PLATFORMS), |tester| {
        case(&mut Fixture::new(tester));
    });
}

/// The mounted gallery, plus the key its navigator is reached through.
struct Fixture<'t> {
    tester: &'t mut WidgetTester,
    navigator: GlobalKey,
}

impl<'t> Fixture<'t> {
    fn new(tester: &'t mut WidgetTester) -> Fixture<'t> {
        // The window at flutter_test's device pixel ratio.
        let ratio = tester.device_pixel_ratio();
        tester
            .view()
            .set_physical_size(Size::new(VIEW[0] * ratio, VIEW[1] * ratio));
        let navigator = GlobalKey::new();
        tester.pump_widget(Gallery::new().navigator_key(navigator.clone()));

        let mut fixture = Fixture { tester, navigator };
        fixture.settle();
        fixture
    }

    /// A second and a half of 20ms frames: past every transition here, and short of a snack
    /// bar's four seconds on screen. A fixed run rather than `pump_and_settle`, since the
    /// progress page's indeterminate indicators ask for frames forever.
    fn settle(&mut self) {
        for _ in 0..75 {
            self.tester.pump_by(Duration::from_millis(20));
        }
    }

    fn navigator(&mut self) -> waft_foundation::Handle<NavigatorState> {
        self.navigator
            .current_state::<NavigatorState>(&self.tester.cell.borrow_mut())
            .expect("the MaterialApp's navigator is mounted")
    }

    /// Pushes `route` over the whole gallery, as `GALLERY_ENTRY` does for a page that fills the
    /// window and a page does for its sub-pages.
    fn push(&mut self, route: RouteRef) {
        let navigator = self.navigator();
        navigator.push(&mut self.tester.cell.borrow_mut(), route);
        self.settle();
    }

    fn push_entry(&mut self, entry: Entry) {
        let route = entry_route(&mut self.tester.cell.borrow_mut(), entry);
        self.push(route);
        self.assert_entry_shown(entry);
    }

    fn push_sub_page(&mut self, entry: Entry, index: usize) {
        let route = sub_route(&mut self.tester.cell.borrow_mut(), entry, index);
        self.push(route);
        self.assert_top_route_settled();
    }

    /// Makes the window `width` wide, at the phone's height, and lets the layout settle.
    fn resize(&mut self, width: f64) {
        let ratio = self.tester.device_pixel_ratio();
        self.tester
            .view()
            .set_physical_size(Size::new(width * ratio, VIEW[1] * ratio));
        self.settle();
    }

    /// Whether a page covers the index: on a phone the index is offstage under any page.
    fn page_open(&mut self) -> bool {
        find::text(INDEX_TITLE).evaluate(self.tester).is_empty()
    }

    /// The system back, as the engine's `popRoute` sends it, and lets the app settle.
    fn system_back(&mut self) {
        {
            let mut app = self.tester.cell.borrow_mut();
            let view = app.platform().views().first().expect("a view").id();
            WidgetsBinding::instance(&mut app).handle_pop_route(&mut app, view);
            app.drain_microtasks();
        }
        self.settle();
    }

    /// Taps the middle of the one onstage widget `finder` finds, and lets the app settle.
    fn tap(&mut self, finder: Finder) {
        self.tester.tap(finder);
        self.settle();
    }

    /// Taps the middle of the one onstage `Text` reading `text`.
    fn tap_text(&mut self, text: &str) {
        self.tap(find::text(text));
    }

    /// Drags a finger from `from` by `by` and lets the app settle. Dart's `dragFrom` stamps
    /// every move at zero, so the drag ends in no fling.
    fn drag(&mut self, from: Offset, by: Offset) {
        self.tester.drag_from(from, by, 1);
        self.settle();
    }

    /// The `Text` data onstage, in tree order: what a failure reports was on screen.
    fn texts(&mut self) -> Vec<String> {
        self.texts_in(find::by_type::<Text>())
    }

    /// The data of the `Text`s `finder` finds.
    fn texts_in(&mut self, finder: Finder) -> Vec<String> {
        let found = finder.evaluate(self.tester);
        found
            .into_iter()
            .filter_map(|text| self.tester.widget(text, |text: &Text| text.data.clone()))
            .collect()
    }

    /// Whether a `Text` containing `text` is mounted, onstage or not.
    fn shows(&mut self, text: &str) -> bool {
        !find::text_containing(text)
            .skip_offstage(false)
            .evaluate(self.tester)
            .is_empty()
    }

    /// The global centre of the topmost onstage `Text` reading exactly `text`.
    fn center_of_text(&mut self, text: &str) -> Offset {
        self.tester.get_center(find::text(text).last())
    }

    /// Checks that a `Text` reading exactly `text` is onstage.
    #[track_caller]
    fn expect_shown(&mut self, text: &str) {
        expect(self.tester, &find::text(text), finds_one_widget());
    }

    /// Checks that no `Text` reading `text` is mounted at all, onstage or not.
    #[track_caller]
    fn expect_gone(&mut self, text: &str) {
        expect(
            self.tester,
            &find::text(text).skip_offstage(false),
            finds_nothing(),
        );
    }

    /// Opens `entry` from its tile on the index and checks its page is shown.
    fn open_from_index(&mut self, entry: Entry) {
        self.scroll_index_to(entry.title());
        self.tap_text(entry.title());
        assert!(self.page_open(), "the tile opened {entry:?} over the index");
        self.assert_entry_shown(entry);
    }

    /// Checks `entry`'s page is shown in full: its app bar wears its title, or, for Reply, which
    /// fills the window and has no app bar over its inbox, its navigation shows the inbox.
    fn assert_entry_shown(&mut self, entry: Entry) {
        if entry.fills_window() {
            self.expect_shown("Inbox");
            return;
        }
        self.assert_app_bar_reads(entry.title());
        self.assert_top_route_settled();
    }

    /// Scrolls the index until the tile reading `title` is built and inside the view, below the
    /// app bar and above the bottom edge: the index is a list longer than the window, so a later
    /// tile sits below the fold, and one the index was scrolled past sits above it.
    fn scroll_index_to(&mut self, title: &str) {
        const TOP: f64 = 80.0;
        const BOTTOM_MARGIN: f64 = 20.0;
        // A tile not built yet is searched for down the list first, then back up it.
        for attempt in 0..40 {
            let up = if self.texts().iter().any(|shown| shown == title) {
                let y = self.center_of_text(title).dy();
                if (TOP..=VIEW[1] - BOTTOM_MARGIN).contains(&y) {
                    return;
                }
                y < TOP
            } else {
                attempt >= 20
            };
            let by = if up { 240.0 } else { -240.0 };
            self.drag(
                Offset::new(VIEW[0] / 2.0, VIEW[1] / 2.0),
                Offset::new(0.0, by),
            );
        }
        panic!("the index never showed {title:?}; saw {:?}", self.texts());
    }

    /// Leaves the top page by its back button, as a user would: the app bar's leading widget.
    fn go_back(&mut self) {
        self.tap(find::by_type::<BackButton>());
        self.assert_top_route_settled();
    }

    /// Checks that the transition into the top route left it in full view: its app bar spans
    /// the top of the view, and nothing between its scaffold and the view fades it.
    ///
    /// The finders skip the routes below the top one, but not a route whose transition settled
    /// part-way, so this checks the top one's geometry and opacity.
    fn assert_top_route_settled(&mut self) {
        let app_bar = find::by_type::<AppBar>();
        assert_eq!(
            (
                self.tester.get_top_left(&app_bar),
                self.tester.get_top_right(&app_bar)
            ),
            (Offset::ZERO, Offset::new(VIEW[0], 0.0)),
            "the top route's app bar spans the top of the view"
        );

        let scaffold = self.tester.render_object(find::by_type::<Scaffold>());
        let app = self.tester.cell.borrow();
        let mut object = Some(scaffold);
        while let Some(current) = object {
            let opacity = if let Some(opacity) = current.downcast::<RenderOpacity>(&app) {
                Some(opacity.opacity(&app))
            } else {
                current
                    .downcast::<RenderAnimatedOpacity>(&app)
                    .map(|opacity| opacity.opacity(&app).value(&app))
            };
            if let Some(opacity) = opacity {
                assert_eq!(
                    opacity, 1.0,
                    "{current:?} above the top route's scaffold fades it"
                );
            }
            object = current.parent(&app);
        }
    }

    /// Checks the top route's app bar reads `title`: its title, or, for a large
    /// `SliverAppBar`, the expanded title under it too, since a `SliverAppBar` builds an
    /// `AppBar`.
    #[track_caller]
    fn assert_app_bar_reads(&mut self, title: &str) {
        expect(
            self.tester,
            &find::descendant(find::by_type::<AppBar>(), find::text(title)),
            finds_widgets(),
        );
    }

    fn brightness(&mut self) -> Brightness {
        let tile = self.tester.element(find::by_type::<ListTile>().first());
        Theme::of(&mut self.tester.cell.borrow_mut(), tile).brightness()
    }
}

#[test]
fn the_index_lists_every_entry() {
    on_each_platform(|fixture| {
        fixture.assert_app_bar_reads("Material Gallery");
        for entry in Entry::ALL {
            fixture.scroll_index_to(entry.title());
            expect(
                fixture.tester,
                &find::widget_with_text::<ListTile>(entry.title()),
                finds_one_widget(),
            );
        }
    });
}

#[test]
fn every_entry_opens_from_its_tile_and_goes_back_with_the_back_button() {
    on_each_platform(|fixture| {
        for entry in Entry::ALL {
            fixture.open_from_index(entry);
            fixture.go_back();
            assert!(
                !fixture.page_open(),
                "the back button on {entry:?} returned to the index"
            );
            fixture.assert_app_bar_reads("Material Gallery");
        }
    });
}

/// Every page builds as a route of its own too, as Reply's does over the whole gallery.
#[test]
fn every_entry_opens_by_a_direct_push() {
    on_each_platform(|fixture| {
        for entry in Entry::ALL {
            fixture.push_entry(entry);
            fixture.go_back();
            assert!(!fixture.page_open(), "{entry:?} came back off");
        }
    });
}

#[test]
fn every_sub_page_builds_and_goes_back() {
    on_each_platform(|fixture| {
        for entry in Entry::ALL {
            for index in 0..entry.sub_page_count() {
                fixture.push_entry(entry);
                fixture.push_sub_page(entry, index);
                fixture.assert_app_bar_reads(entry.sub_title(index));
                fixture.go_back();
                fixture.assert_app_bar_reads(entry.title());
                fixture.go_back();
            }
        }
        assert!(!fixture.page_open(), "every sub-page came back off");
    });
}

/// The index's drawer lists the entries too, and a tile closes it before it pushes.
#[test]
fn the_index_drawer_opens_an_entry() {
    on_each_platform(|fixture| {
        fixture.tap(find::by_type::<DrawerButton>());
        expect(
            fixture.tester,
            &find::by_type::<DrawerHeader>(),
            finds_one_widget(),
        );

        fixture.tap(find::descendant(
            find::by_type::<Drawer>(),
            find::widget_with_text::<ListTile>(Entry::Lists.title()),
        ));
        fixture.assert_app_bar_reads(Entry::Lists.title());
        fixture.assert_top_route_settled();
        fixture.go_back();
        assert!(!fixture.page_open(), "the Lists page came back off");
        expect(
            fixture.tester,
            &find::by_type::<DrawerHeader>().skip_offstage(false),
            finds_nothing(),
        );
    });
}

/// The toggle pins the theme mode to the brightness the index is not showing, and back.
#[test]
fn the_theme_toggle_flips_the_brightness() {
    on_each_platform(|fixture| {
        assert_eq!(
            fixture.brightness(),
            Brightness::Light,
            "the test platform is light, and the app follows it"
        );

        fixture.tap(find::by_tooltip("Use the dark theme"));
        assert_eq!(fixture.brightness(), Brightness::Dark);

        fixture.tap(find::by_tooltip("Use the light theme"));
        assert_eq!(fixture.brightness(), Brightness::Light);
    });
}

/// The scaffold's own floating action button: alone in its slot, so the tap is its own.
///
/// A button's `_InputPadding` hit-tests every position it is handed, as Dart's does; the
/// `Semantics` around it keeps it to its own box, so a tap in a row of buttons reaches the one
/// under it.
#[test]
fn a_button_reports_its_press() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Buttons);
        fixture.expect_shown("Last pressed: \u{2014}");

        fixture.tap_text("Compose");
        fixture.expect_shown("Last pressed: Scaffold FAB");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// The end drawer opens from its button, and a tile closes it. The button is the last of the
/// app bar's actions; the drawer button before it is
/// [`the_drawer_opens_from_its_app_bar_action`].
#[test]
fn the_end_drawer_opens_from_the_app_bar_and_closes_from_a_tile() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Drawers);

        fixture.tap(find::by_type::<EndDrawerButton>());
        fixture.expect_shown(END_DRAWER_HEADER);
        fixture.tap_text("Starred only");
        fixture.expect_gone(END_DRAWER_HEADER);
        fixture.expect_shown("Chose Starred only");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// A swipe in from the start edge opens the drawer on Android. On macOS a `MaterialPageRoute`
/// slides as Cupertino's does, and the same swipe is its back gesture: the detector for it
/// sits over the page, so its recognizer joins the arena before the drawer's and wins it.
#[test]
fn the_start_edge_swipe_opens_the_drawer_or_goes_back() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Drawers);
        let platform = fixture.tester.target_platform();

        fixture.drag(Offset::new(4.0, VIEW[1] / 2.0), Offset::new(280.0, 0.0));
        if platform == TargetPlatform::MacOS {
            assert!(!fixture.page_open(), "the swipe went back to the index");
            return;
        }
        fixture.expect_shown(DRAWER_HEADER);
        fixture.tap_text("Outbox");
        fixture.expect_gone(DRAWER_HEADER);
        fixture.expect_shown("Chose Outbox");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// The close-icon snack bar, the last button of its row, and its close icon, the one button
/// of the snack bar.
#[test]
fn a_snack_bar_shows_and_closes_from_its_icon() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Messages);

        fixture.tap_text("With close icon");
        fixture.expect_shown("A snack bar with a close icon");
        fixture.tap(find::descendant(
            find::by_type::<SnackBar>(),
            find::by_type::<IconButton>(),
        ));
        fixture.expect_gone("A snack bar with a close icon");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// The persistent sheet, the last button of its row, closed by the sheet's one button.
#[test]
fn a_persistent_bottom_sheet_opens_and_closes() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Messages);

        fixture.tap_text("Persistent sheet");
        fixture.expect_shown(PERSISTENT_SHEET_TEXT);
        fixture.tap_text("Close persistent sheet");
        fixture.expect_gone(PERSISTENT_SHEET_TEXT);

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// The large app bar shrinks from its expanded height to its collapsed one as the list rises
/// under it: Material 3's 152 and 64 points.
#[test]
fn scrolling_collapses_the_large_app_bar() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::AppBars);
        assert_eq!(app_bar_extent(fixture), 152.0, "the bar starts expanded");

        let middle = Offset::new(VIEW[0] / 2.0, VIEW[1] / 2.0);
        fixture.drag(middle, Offset::new(0.0, -300.0));
        assert_eq!(
            app_bar_extent(fixture),
            64.0,
            "the bar collapsed to its toolbar"
        );

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// How much of the view the top page's `SliverAppBar` covers.
fn app_bar_extent(fixture: &mut Fixture) -> f64 {
    let app_bar = fixture
        .tester
        .render_object(find::by_type::<SliverAppBar>());
    let app = fixture.tester.cell.borrow();
    app_bar
        .as_sliver()
        .expect("a SliverAppBar renders a sliver")
        .geometry(&app)
        .paint_extent
}

#[test]
fn the_floating_app_bar_opens_from_the_large_one() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::AppBars);
        fixture.tap_text(Entry::AppBars.sub_title(0));
        fixture.assert_app_bar_reads(Entry::AppBars.sub_title(0));

        fixture.go_back();
        fixture.assert_app_bar_reads(Entry::AppBars.title());
        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

#[test]
fn tapping_a_tile_selects_it() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Lists);
        fixture.expect_shown("Selected: Inbox");

        fixture.tap_text("Starred");
        fixture.expect_shown("Selected: Starred");
        // The index under the page is offstage, with the Lists tile selected.
        let tiles = find::by_type::<ListTile>().evaluate(fixture.tester);
        let selected = tiles
            .into_iter()
            .filter(|tile| {
                fixture
                    .tester
                    .widget(*tile, |tile: &ListTile| tile.selected)
            })
            .count();
        assert_eq!(selected, 1, "one tile is selected");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// The banner's button is alone in its row, and its Dismiss action alone in the banner's.
#[test]
fn a_banner_shows_and_dismisses() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Messages);

        fixture.tap_text("Show banner");
        fixture.expect_shown(BANNER_TEXT);
        fixture.tap_text("Dismiss");
        fixture.expect_gone(BANNER_TEXT);

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// Taps the `W` reading `label` in the row of common buttons whose first one reads `row`, and
/// answers what the page's last-pressed line reads afterwards, if it does not read `expected`.
///
/// The row narrows the finder: the Filled and Tonal rows each have a `FilledButton` reading
/// "Icon".
fn press_common_button<W: 'static>(
    fixture: &mut Fixture,
    row: &str,
    label: &str,
    expected: &str,
) -> Option<String> {
    fixture.tap(find::descendant(
        find::widget_with_text::<Wrap>(row),
        find::widget_with_text::<W>(label),
    ));
    let pressed = format!("Last pressed: {expected}");
    if !find::text(&pressed).evaluate(fixture.tester).is_empty() {
        return None;
    }
    let shown = fixture
        .texts()
        .into_iter()
        .find(|text| text.starts_with("Last pressed: "));
    Some(format!("{expected}: the page reads {shown:?}"))
}

/// Every enabled common button on the Buttons page, tapped in the middle as a user would.
#[test]
fn each_common_button_reports_its_press() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Buttons);
        let missed: Vec<String> = [
            press_common_button::<ElevatedButton>(fixture, "Elevated", "Elevated", "Elevated"),
            press_common_button::<ElevatedButton>(fixture, "Elevated", "Icon", "Elevated icon"),
            press_common_button::<FilledButton>(fixture, "Filled", "Filled", "Filled"),
            press_common_button::<FilledButton>(fixture, "Filled", "Icon", "Filled icon"),
            press_common_button::<FilledButton>(fixture, "Tonal", "Tonal", "Tonal"),
            press_common_button::<FilledButton>(fixture, "Tonal", "Icon", "Tonal icon"),
            press_common_button::<OutlinedButton>(fixture, "Outlined", "Outlined", "Outlined"),
            press_common_button::<OutlinedButton>(fixture, "Outlined", "Icon", "Outlined icon"),
            press_common_button::<TextButton>(fixture, "Text", "Text", "Text"),
            press_common_button::<TextButton>(fixture, "Text", "Icon", "Text icon"),
        ]
        .into_iter()
        .flatten()
        .collect();
        assert!(missed.is_empty(), "presses that did not land: {missed:#?}");
    });
}

/// The first three icon-button toggles of the row.
#[test]
fn the_first_icon_button_toggles_turn_on() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Buttons);
        // FlutterTest's square glyphs wrap each row of common buttons onto two runs, which
        // puts the toggles below the fold: scroll them up, as a finger would.
        fixture.drag(
            Offset::new(VIEW[0] / 2.0, VIEW[1] / 2.0),
            Offset::new(0.0, -VIEW[1] / 4.0),
        );
        let mut missed = Vec::new();
        for variant in ["Standard", "Filled", "Tonal"] {
            fixture.tap(find::by_tooltip(variant));
            let summary = fixture
                .texts()
                .into_iter()
                .find(|text| text.starts_with("Selected: "));
            if !summary
                .as_deref()
                .is_some_and(|summary| summary.contains(variant))
            {
                missed.push(format!("{variant}: the page reads {summary:?}"));
            }
        }
        assert!(
            missed.is_empty(),
            "toggles that did not turn on: {missed:#?}"
        );
    });
}

/// The drawer button, the first of the Drawers page's two app bar actions.
#[test]
fn the_drawer_opens_from_its_app_bar_action() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Drawers);
        fixture.tap(find::by_type::<DrawerButton>());
        fixture.expect_shown(DRAWER_HEADER);
    });
}

/// Taps the button reading `label` on the Messages page and checks `shown` comes up.
fn assert_messages_button_shows(fixture: &mut Fixture, label: &str, shown: &str) {
    fixture.open_from_index(Entry::Messages);
    fixture.tap_text(label);
    fixture.expect_shown(shown);
}

#[test]
fn the_fixed_snack_bar_shows_from_its_button() {
    on_each_platform(|fixture| {
        assert_messages_button_shows(fixture, "Fixed", "A fixed snack bar");
    });
}

#[test]
fn the_floating_snack_bar_shows_from_its_button() {
    on_each_platform(|fixture| {
        assert_messages_button_shows(fixture, "Floating", "A floating snack bar");
    });
}

#[test]
fn the_snack_bar_with_an_action_shows_from_its_button() {
    on_each_platform(|fixture| {
        assert_messages_button_shows(fixture, "With action", "Message archived");
    });
}

#[test]
fn the_modal_bottom_sheet_opens_from_its_button() {
    on_each_platform(|fixture| {
        assert_messages_button_shows(fixture, "Modal sheet", "A modal bottom sheet");
    });
}

/// A filter chip toggles on a tap, and the page's caption says which are on.
#[test]
fn a_filter_chip_toggles_on_a_tap() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Chips);
        assert!(!fixture.shows("filters on: Coffee"));

        fixture.tap_text("Coffee");
        assert!(
            fixture.shows("filters on: Coffee"),
            "the filter chip reported its selection; saw {:?}",
            fixture.texts()
        );

        fixture.tap_text("Coffee");
        assert!(
            !fixture.shows("filters on: Coffee"),
            "a second tap cleared it"
        );
    });
}

/// An alert dialog opens from its button and answers the page with the button it was closed by.
#[test]
fn an_alert_dialog_answers_with_its_button() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Dialogs);
        fixture.tap_text("Alert dialog");
        assert!(fixture.shows("Discard draft?"), "saw {:?}", fixture.texts());

        fixture.tap_text("Discard");
        assert!(
            fixture.shows("Answer: Discard"),
            "the dialog popped with its answer; saw {:?}",
            fixture.texts()
        );
        assert!(!fixture.shows("Discard draft?"), "the dialog closed");
    });
}

/// A collapsed expansion tile opens on a tap and shows its children.
#[test]
fn an_expansion_tile_opens_on_a_tap() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Cards);
        let shown = |fixture: &mut Fixture| {
            fixture
                .texts()
                .iter()
                .filter(|text| text.as_str() == EXPANDED_TILE_CHILD)
                .count()
        };
        let before = shown(fixture);
        fixture.tap_text("Collapsed tile");
        assert_eq!(shown(fixture), before + 1);
    });
}

/// The time picker opens in its dial, and OK answers the page with the time it showed.
#[test]
fn the_time_picker_opens_and_answers_with_ok() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Pickers);
        let before = fixture
            .texts()
            .into_iter()
            .find(|text| text.starts_with("Time: "));

        fixture.tap_text("Dial");
        assert!(fixture.shows("Select time"), "saw {:?}", fixture.texts());

        fixture.tap_text("OK");
        assert!(!fixture.shows("Select time"), "the dialog closed");
        let after = fixture
            .texts()
            .into_iter()
            .find(|text| text.starts_with("Time: "));
        assert_ne!(before, after, "the page's caption took the picked time");
    });
}

/// The first autocomplete lists the fruits that contain the text, and a tap on one selects it.
#[test]
fn an_autocomplete_filters_its_options_and_selects_one() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Autocomplete);
        expect(
            fixture.tester,
            &find::text("Selected: \u{2014}"),
            finds_widgets(),
        );

        fixture.tap(find::by_type::<waft_material::TextFormField>().first());
        fixture.expect_shown("Apple");

        fixture
            .tester
            .enter_text(find::by_type::<waft_widgets::EditableText>().at(0), "cher");
        fixture.settle();
        fixture.expect_gone("Apple");
        fixture.tap_text("Cherry");
        fixture.expect_shown("Selected: Cherry");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// The second autocomplete's options are people, listed and selected by their names.
#[test]
fn an_autocomplete_of_a_custom_type_shows_the_display_string() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Autocomplete);

        fixture.tap(find::by_type::<waft_material::TextFormField>().at(1));
        fixture
            .tester
            .enter_text(find::by_type::<waft_widgets::EditableText>().at(1), "bob");
        fixture.settle();
        fixture.tap_text("Bob");
        fixture.expect_shown("Selected: Bob");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// Presses at `from`, holds for `hold`, drags by `by` and lets go, a frame after each step as a
/// finger's would take, and lets the app settle: what a reorder needs to see the item move.
fn press_hold_drag_release(fixture: &mut Fixture, from: Offset, hold: Duration, by: Offset) {
    let mut pointer = waft_widgets::testing::TestPointer::touch(fixture.tester.next_pointer());
    fixture.tester.down(&mut pointer, from);
    fixture.tester.pump_by(hold);
    fixture.tester.move_by(&mut pointer, by, true);
    fixture.tester.pump_by(waft_gestures::K_PRESS_TIMEOUT);
    fixture.tester.up(&pointer);
    fixture.settle();
}

/// A tile is dragged by the handle the page placed in it, past the last tile: a reorder puts the item last only once its leading edge is below the last tile.
#[test]
fn a_tile_is_reordered_by_its_drag_handle() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::ReorderableListView);
        fixture.expect_shown("Order: Red, Green, Blue");

        let handle = find::descendant(
            find::widget_with_text::<ListTile>("Red"),
            find::by_icon(&waft_material::Icons::drag_handle()),
        );
        let from = fixture.tester.get_center(handle);
        press_hold_drag_release(
            fixture,
            from,
            waft_gestures::K_PRESS_TIMEOUT,
            Offset::new(0.0, 200.0),
        );
        fixture.expect_shown("Order: Green, Blue, Red");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// A tile is pressed and held, and then dragged past the last tile.
#[test]
fn a_tile_is_reordered_by_a_long_press() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::ReorderableListView);
        fixture.expect_shown("Order: Cat, Dog, Fox");

        let from = fixture.center_of_text("Cat");
        press_hold_drag_release(
            fixture,
            from,
            waft_gestures::K_LONG_PRESS_TIMEOUT + waft_gestures::K_PRESS_TIMEOUT,
            Offset::new(0.0, 200.0),
        );
        fixture.expect_shown("Order: Dog, Fox, Cat");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// A data table's column header sorts it (a second tap reverses it), and a row's tap selects it.
#[test]
fn a_data_table_sorts_by_its_header_and_selects_a_row() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::DataTables);
        fixture.expect_shown("Sorted by Dessert ascending; 0 selected");
        assert_eq!(first_dessert(fixture), "Cupcake");

        fixture.tap(find::text("Dessert").first());
        fixture.expect_shown("Sorted by Dessert descending; 0 selected");
        assert_eq!(first_dessert(fixture), "Ice cream sandwich");

        fixture.tap(find::text("Dessert").first());
        fixture.expect_shown("Sorted by Dessert ascending; 0 selected");

        fixture.tap_text("Cupcake");
        fixture.expect_shown("Sorted by Dessert ascending; 1 selected");
    });
}

/// The first dessert row the table shows.
fn first_dessert(fixture: &mut Fixture) -> String {
    fixture
        .texts()
        .into_iter()
        .find(|text| DESSERT_NAMES.contains(&text.as_str()))
        .expect("a dessert row is shown")
}

const DESSERT_NAMES: [&str; 6] = [
    "Frozen yogurt",
    "Ice cream sandwich",
    "Eclair",
    "Cupcake",
    "Gingerbread",
    "Honeycomb",
];

/// The search bar's view opens from a tap, lists the suggestions, and a tapped suggestion
/// closes it and reaches the page.
#[test]
fn a_search_anchor_view_returns_the_chosen_suggestion() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Search);
        fixture.expect_shown("Chose: \u{2014}");

        fixture.tap(find::text("Pick a fruit"));
        fixture.tap_text("Banana");
        fixture.expect_shown("Chose: Banana");
    });
}

/// Tapping a carousel item reports its index in the caption under that carousel.
#[test]
fn a_carousel_item_reports_its_tap() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Carousels);

        fixture.tap(find::text("Item 1").first());
        assert!(
            fixture.texts().iter().any(|text| text == "Tapped item 1"),
            "the first carousel reported item 1; saw {:?}",
            fixture.texts()
        );
    });
}

/// Reply opens a mail as a page on a phone, and its back, or the system's, returns to the list.
#[test]
fn reply_opens_a_mail_as_a_page_and_back_returns_to_the_list() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Adaptive);
        fixture.expect_shown("Brunch this weekend?");

        fixture.tap_text("Brunch this weekend?");
        fixture.expect_shown("7 Messages");

        fixture.tap(find::by_tooltip("Back"));
        fixture.expect_gone("7 Messages");
        fixture.expect_shown("Brunch this weekend?");

        // The system back closes the mail, not the whole app.
        fixture.tap_text("Bonjour from Paris");
        fixture.expect_shown("7 Messages");
        fixture.system_back();
        fixture.expect_gone("7 Messages");
        fixture.expect_shown("Inbox");
    });
}

/// On a wide window Reply shows the open mail beside the list, a rail instead of the bar, and
/// opening another mail changes only the detail.
#[test]
fn reply_on_a_wide_window_shows_the_mail_beside_the_list() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Adaptive);
        fixture.resize(1000.0);
        expect(
            fixture.tester,
            &find::by_type::<NavigationRail>(),
            finds_one_widget(),
        );
        fixture.expect_shown("7 Messages");

        fixture.tap_text("Bonjour from Paris");
        expect(
            fixture.tester,
            &find::descendant(find::by_type::<AppBar>(), find::text("Bonjour from Paris")),
            finds_one_widget(),
        );
        fixture.expect_shown("Brunch this weekend?");
        // Only the way back to the gallery: the mail beside the list has no back of its own.
        expect(
            fixture.tester,
            &find::by_type::<BackButton>(),
            finds_one_widget(),
        );
    });
}

/// On a wide window the index stays beside the page it opens, and opening another page
/// replaces it there.
#[test]
fn a_wide_window_shows_the_index_beside_the_page() {
    on_each_platform(|fixture| {
        fixture.resize(1300.0);
        fixture.expect_shown("No page open");

        fixture.tap_text(Entry::Buttons.title());
        fixture.assert_app_bar_reads(Entry::Buttons.title());
        fixture.assert_app_bar_reads(INDEX_TITLE);
        expect(
            fixture.tester,
            &find::by_type::<BackButton>(),
            finds_nothing(),
        );

        fixture.tap_text(Entry::Lists.title());
        fixture.assert_app_bar_reads(Entry::Lists.title());
        expect(
            fixture.tester,
            &find::descendant(
                find::by_type::<AppBar>(),
                find::text(Entry::Buttons.title()),
            ),
            finds_nothing(),
        );
    });
}

/// The stepper page's continue button moves the vertical stepper on and says so.
#[test]
fn the_stepper_page_continues_to_the_next_step() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Steppers);
        fixture.expect_shown(LAST_ACTION);

        // The vertical stepper comes first, and the horizontal one has a continue button too.
        fixture.tap(find::text("Continue").first());
        fixture.expect_shown("Continued to step 2");

        fixture.go_back();
        assert!(!fixture.page_open());
    });
}

/// Tapping an expansion panel's header opens it.
#[test]
fn an_expansion_panel_opens_on_a_header_tap() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::ExpansionPanels);
        fixture.expect_shown("Payment (closed)");

        fixture.tap_text("Payment (closed)");
        fixture.expect_shown("Payment (open)");
    });
}

/// The account header shows the signed-in account's name and email.
#[test]
fn the_account_header_shows_the_account() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::AccountHeader);
        fixture.expect_shown("Jane Doe");
        fixture.expect_shown("jane@example.com");
    });
}

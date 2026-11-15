//! The gallery's navigation, driven through the real pages.
//!
//! These tests boot the same app `main.rs` boots, so a page that panics while building, a
//! catalog arm that names a missing sub-page, or a page whose app bar loses its title or its
//! back button all fail here. Every case runs on Android, the platform Flutter's own tests
//! run on, and then on macOS, where `MaterialPageRoute` slides as Cupertino's does and the
//! scroll views wear desktop scrollbars.

use std::rc::Rc;
use std::time::Duration;

use material_gallery_lib::pages::drawers::{DRAWER_HEADER, END_DRAWER_HEADER};
use material_gallery_lib::pages::messages::{BANNER_TEXT, PERSISTENT_SHEET_TEXT};
use material_gallery_lib::{Entry, Gallery, entry_route, sub_route};
use waft_embedder::{
    Brightness, Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind,
    TargetPlatform,
};
use waft_foundation::{AppCell, Handle};
use waft_gestures::GestureBinding;
use waft_material::{
    AppBar, BackButton, Drawer, DrawerButton, DrawerHeader, ElevatedButton, EndDrawerButton,
    FilledButton, IconButton, ListTile, OutlinedButton, Scaffold, SliverAppBar, SnackBar,
    TextButton, Theme, install_material_icon_font,
};
use waft_painting::PaintingBinding;
use waft_rendering::{
    AnyRenderObject, RenderAnimatedOpacity, RenderAnimatedOpacityMixin, RenderOpacity,
    RenderParagraph, RendererBinding,
};
use waft_scheduler::SchedulerBinding;
use waft_test::{IMPLICIT_VIEW_ID, TestPlatform, TestView};
use waft_widgets::{
    AnyElement, AnyRoute, GlobalKey, IntoWidget, NavigatorState, Text, WidgetsBinding,
    downcast_widget, run_app,
};

/// The window `main.rs` opens, in logical points.
const VIEW: [f64; 2] = [420.0, 720.0];

/// A phone, and a desktop.
const PLATFORMS: [TargetPlatform; 2] = [TargetPlatform::Android, TargetPlatform::MacOS];

/// The stack a platform's main thread gives the framework: 8 MiB. A debug build of
/// `ThemeData` alone takes over 1 MiB of it, and the gallery builds two, which does not fit
/// the 2 MiB a test thread has.
const MAIN_THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;

/// Runs `case` against a fresh gallery on each of [`PLATFORMS`].
fn on_each_platform(case: fn(&mut Fixture)) {
    for platform in PLATFORMS {
        on_platform(platform, case);
    }
}

/// Runs `case` against a fresh gallery on `platform`, on a thread with a main thread's stack,
/// re-raising its panic with the platform it failed on.
fn on_platform(platform: TargetPlatform, case: fn(&mut Fixture)) {
    let joined = std::thread::Builder::new()
        .name(format!("{platform:?}"))
        .stack_size(MAIN_THREAD_STACK_SIZE)
        .spawn(move || case(&mut Fixture::new(platform)))
        .expect("a thread for the case")
        .join();
    if let Err(panic) = joined {
        eprintln!("the case failed on {platform:?}");
        std::panic::resume_unwind(panic);
    }
}

/// The mounted gallery, plus the key its navigator is reached through.
struct Fixture {
    cell: Rc<AppCell>,
    navigator: GlobalKey,
    at: Duration,
    /// Where the last pointer event was, for the next move's delta.
    last_point: Offset,
}

impl Fixture {
    fn new(platform: TargetPlatform) -> Fixture {
        // A 420x720 view at 1x, so `run_app` finds one to attach to and a logical point is a
        // physical pixel.
        let platform = TestPlatform::new()
            .on(platform)
            .with_view(Rc::new(TestView::new(VIEW[0], VIEW[1])));
        let cell = AppCell::with_platform(Rc::new(platform));
        let mut app = cell.borrow_mut();
        // What the shell does at start-up: the app-wide fonts every paragraph shapes against.
        PaintingBinding::instance(&mut app).install_fonts(&mut app, |fonts| {
            fonts.add_source(valo_system_fonts::SystemFonts::load());
        });

        let navigator = GlobalKey::new();
        run_app(
            &mut app,
            Gallery::new()
                .navigator_key(navigator.clone())
                .into_widget(),
        );
        // The same ordering constraint the binary has: `run_app` is what installs the font
        // collection the icon face is registered into.
        install_material_icon_font(&mut app);
        // `run_app` attaches the root widget on the next timer turn, as Dart's `Timer.run` does.
        drop(app);
        cell.elapse(Duration::ZERO);

        let mut fixture = Fixture {
            cell,
            navigator,
            at: Duration::ZERO,
            last_point: Offset::ZERO,
        };
        fixture.settle();
        fixture
    }

    /// Pumps a fixed span of frames: 1.5 s, past the longest transition here (Android's page
    /// transition), and short of a snack bar's four seconds on screen.
    ///
    /// Each frame phase ends with the checkpoint the shell runs after every platform event, so
    /// a continuation awaiting a route's result resumes as it would in the app.
    fn settle(&mut self) {
        for _ in 0..75 {
            self.pump(Duration::from_millis(20));
        }
    }

    /// A frame of the app's view `step` after the last.
    fn pump(&mut self, step: Duration) {
        self.at += step;
        let views = [IMPLICIT_VIEW_ID];
        SchedulerBinding::handle_begin_frame(&mut self.cell.borrow_mut(), &views, Some(self.at));
        self.cell.checkpoint();
        SchedulerBinding::handle_draw_frame(&mut self.cell.borrow_mut(), &views);
        self.cell.checkpoint();
    }

    fn navigator(&mut self) -> Handle<NavigatorState> {
        self.navigator
            .current_state::<NavigatorState>(&self.cell.borrow_mut())
            .expect("the MaterialApp's navigator is mounted")
    }

    /// The same call `GALLERY_ENTRY` makes.
    fn push(&mut self, route: AnyRoute) {
        let navigator = self.navigator();
        navigator.push(&mut self.cell.borrow_mut(), route);
        self.settle();
        self.assert_top_route_settled();
    }

    fn can_pop(&mut self) -> bool {
        let navigator = self.navigator();
        navigator.can_pop(&self.cell.borrow())
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
        self.subtree(root)
    }

    /// `root` and its descendants, in tree order.
    fn subtree(&mut self, root: AnyElement) -> Vec<AnyElement> {
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

    /// Whether any `W` is mounted.
    fn has<W: 'static>(&mut self) -> bool {
        let elements = self.elements();
        elements.into_iter().any(|element| self.is::<W>(element))
    }

    /// Whether `element` was built from a `W`.
    fn is<W: 'static>(&self, element: AnyElement) -> bool {
        downcast_widget::<W>(&**element.widget(&self.cell.borrow())).is_some()
    }

    /// The last `W` in tree order: the one on the top route, since every route in the stack
    /// keeps its elements and the overlay builds the top one last.
    fn last<W: 'static>(&mut self) -> AnyElement {
        let elements = self.elements();
        elements
            .into_iter()
            .rev()
            .find(|element| self.is::<W>(*element))
            .unwrap_or_else(|| panic!("no {} is mounted", std::any::type_name::<W>()))
    }

    /// The last `W` under `ancestor`.
    fn last_in<W: 'static>(&mut self, ancestor: AnyElement) -> AnyElement {
        let elements = self.subtree(ancestor);
        elements
            .into_iter()
            .rev()
            .find(|element| self.is::<W>(*element))
            .unwrap_or_else(|| panic!("no {} under {ancestor:?}", std::any::type_name::<W>()))
    }

    /// The `ListTile` under `ancestor` whose texts include `title`.
    fn tile_in(&mut self, ancestor: AnyElement, title: &str) -> AnyElement {
        let tiles: Vec<AnyElement> = self
            .subtree(ancestor)
            .into_iter()
            .filter(|element| self.is::<ListTile>(*element))
            .collect();
        tiles
            .into_iter()
            .find(|tile| self.texts_in(*tile).iter().any(|text| text == title))
            .unwrap_or_else(|| panic!("no ListTile reading {title:?} under {ancestor:?}"))
    }

    /// The first `W` in tree order whose texts include `label`.
    fn button_reading<W: 'static>(&mut self, label: &str) -> AnyElement {
        let elements = self.elements();
        let buttons: Vec<AnyElement> = elements
            .into_iter()
            .filter(|element| self.is::<W>(*element))
            .collect();
        buttons
            .into_iter()
            .find(|button| self.texts_in(*button).iter().any(|text| text == label))
            .unwrap_or_else(|| panic!("no {} reading {label:?}", std::any::type_name::<W>()))
    }

    /// The `IconButton` whose tooltip is `tooltip`.
    fn icon_button_with_tooltip(&mut self, tooltip: &str) -> AnyElement {
        let elements = self.elements();
        elements
            .into_iter()
            .find(|element| {
                let app = self.cell.borrow();
                let widget = element.widget(&app);
                downcast_widget::<IconButton>(&**widget)
                    .is_some_and(|button| button.tooltip.as_deref() == Some(tooltip))
            })
            .unwrap_or_else(|| panic!("no IconButton with the tooltip {tooltip:?}"))
    }

    /// The `Text` data under `ancestor`, in tree order.
    fn texts_in(&mut self, ancestor: AnyElement) -> Vec<String> {
        let elements = self.subtree(ancestor);
        elements
            .into_iter()
            .filter_map(|element| {
                downcast_widget::<Text>(&**element.widget(&self.cell.borrow()))
                    .and_then(|text| text.data.clone())
            })
            .collect()
    }

    /// What the top route's app bar reads: its title, and, for a large `SliverAppBar`, the
    /// expanded title under it.
    ///
    /// A `SliverAppBar` builds an `AppBar`, so this one lookup serves both.
    fn app_bar_texts(&mut self) -> Vec<String> {
        let app_bar = self.last::<AppBar>();
        self.texts_in(app_bar)
    }

    /// Every render object of the mounted tree, parents before children: the render tree of
    /// the one view the app shows.
    fn render_objects(&mut self) -> Vec<AnyRenderObject> {
        let root = {
            let mut app = self.cell.borrow_mut();
            let render_views = RendererBinding::instance(&mut app).render_views(&app);
            let [render_view] = render_views[..] else {
                panic!("the app shows one view, not {}", render_views.len());
            };
            render_view.as_object()
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

    /// The plain text of every paragraph laid out in the view.
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

    /// The view-space centre of `element`'s render box.
    fn center_of(&mut self, element: AnyElement) -> Offset {
        let app = self.cell.borrow();
        let object = element
            .render_object(&app)
            .and_then(AnyRenderObject::as_box)
            .unwrap_or_else(|| panic!("{element:?} has no render box"));
        let size = object.size(&app);
        let origin = object.local_to_global(&app, Offset::ZERO, None);
        origin + Offset::new(size.width() / 2.0, size.height() / 2.0)
    }

    /// The view-space centre of the topmost paragraph reading exactly `text`.
    fn center_of_text(&mut self, text: &str) -> Offset {
        let mut elements = self.elements();
        elements.reverse();
        for element in elements {
            let paragraph = element
                .render_object(&self.cell.borrow())
                .and_then(|object| object.downcast::<RenderParagraph>(&self.cell.borrow()));
            let Some(paragraph) = paragraph else {
                continue;
            };
            if paragraph
                .text(&self.cell.borrow())
                .to_plain_text(true, true)
                == text
            {
                return self.center_of(element);
            }
        }
        panic!("nothing on screen reads {text:?}; saw {:?}", self.texts())
    }

    /// Taps the middle of the topmost thing reading `text`.
    fn tap_text(&mut self, text: &str) {
        let point = self.center_of_text(text);
        self.tap_at(point);
    }

    /// Taps the middle of `element`.
    fn tap(&mut self, element: AnyElement) {
        let point = self.center_of(element);
        self.tap_at(point);
    }

    fn tap_at(&mut self, point: Offset) {
        self.send(PointerChange::Down, point);
        self.at += Duration::from_millis(20);
        self.send(PointerChange::Up, point);
        self.settle();
    }

    /// A finger dragged from `from` by `by`, slowly enough that it ends in no fling.
    fn drag(&mut self, from: Offset, by: Offset) {
        const STEPS: u32 = 20;
        self.send(PointerChange::Down, from);
        for step in 1..=STEPS {
            self.at += Duration::from_millis(16);
            let at_point = from + by * (f64::from(step) / f64::from(STEPS));
            self.send(PointerChange::Move, at_point);
            self.pump(Duration::ZERO);
        }
        // Held still before the lift, so the velocity tracker reads a stop.
        self.at += Duration::from_millis(200);
        self.send(PointerChange::Move, from + by);
        self.send(PointerChange::Up, from + by);
        self.settle();
    }

    /// Delivers one touch event at `at_point`, with the delta from the last one the way an
    /// engine reports it: the converter reads a move's delta from the packet.
    fn send(&mut self, change: PointerChange, at_point: Offset) {
        let delta = match change {
            PointerChange::Move => at_point - self.last_point,
            _ => Offset::ZERO,
        };
        self.last_point = at_point;
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
                    physical_delta_x: delta.dx(),
                    physical_delta_y: delta.dy(),
                    ..PointerData::default()
                }]),
            );
        }
        self.cell.checkpoint();
    }

    /// Opens `entry` from its tile on the index and checks the page's app bar wears its title.
    fn open_from_index(&mut self, entry: Entry) {
        self.tap_text(entry.title());
        assert!(self.can_pop(), "the tile pushed {entry:?} over the index");
        self.assert_app_bar_reads(entry.title());
        self.assert_top_route_settled();
    }

    /// Leaves the top page by its back button, as a user would: the app bar's leading widget.
    fn go_back(&mut self) {
        let back = self.last::<BackButton>();
        self.tap(back);
        self.assert_top_route_settled();
    }

    /// Checks that the transition into the top route left it in full view: its app bar spans
    /// the top of the view, and nothing between its scaffold and the view fades it.
    ///
    /// The text and tap helpers read the render tree whatever its opacity or offset, so a
    /// transition that settled part-way would pass them.
    fn assert_top_route_settled(&mut self) {
        let app_bar = self.last::<AppBar>();
        let (top_left, top_right) = {
            let app = self.cell.borrow();
            let object = app_bar
                .render_object(&app)
                .and_then(AnyRenderObject::as_box)
                .expect("an AppBar renders a box");
            let width = object.size(&app).width();
            (
                object.local_to_global(&app, Offset::ZERO, None),
                object.local_to_global(&app, Offset::new(width, 0.0), None),
            )
        };
        assert_eq!(
            (top_left, top_right),
            (Offset::ZERO, Offset::new(VIEW[0], 0.0)),
            "the top route's app bar spans the top of the view"
        );

        let scaffold = self.last::<Scaffold>();
        let app = self.cell.borrow();
        let mut object = scaffold.render_object(&app);
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

    fn assert_app_bar_reads(&mut self, title: &str) {
        let texts = self.app_bar_texts();
        assert!(
            texts.iter().any(|text| text == title),
            "the top app bar should read {title:?}; it reads {texts:?}"
        );
    }

    fn brightness(&mut self) -> Brightness {
        let tile = self.last::<ListTile>();
        Theme::of(&mut self.cell.borrow_mut(), tile).brightness()
    }
}

#[test]
fn the_index_lists_every_entry() {
    on_each_platform(|fixture| {
        fixture.assert_app_bar_reads("Material Gallery");
        let texts = fixture.texts();
        for entry in Entry::ALL {
            assert!(
                texts.iter().any(|text| text == entry.title()),
                "the index has no tile for {entry:?}; saw {texts:?}"
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
                !fixture.can_pop(),
                "the back button on {entry:?} returned to the index"
            );
            fixture.assert_app_bar_reads("Material Gallery");
        }
    });
}

/// `GALLERY_ENTRY` pushes the entry's route without a tap: every page builds that way too.
#[test]
fn every_entry_opens_by_a_direct_push() {
    on_each_platform(|fixture| {
        for entry in Entry::ALL {
            let route = entry_route(&mut fixture.cell.borrow_mut(), entry);
            fixture.push(route);
            fixture.assert_app_bar_reads(entry.title());
            fixture.go_back();
            assert!(!fixture.can_pop(), "{entry:?} came back off");
        }
    });
}

#[test]
fn every_sub_page_builds_and_goes_back() {
    on_each_platform(|fixture| {
        for entry in Entry::ALL {
            for index in 0..entry.sub_page_count() {
                let route = entry_route(&mut fixture.cell.borrow_mut(), entry);
                fixture.push(route);
                let route = sub_route(&mut fixture.cell.borrow_mut(), entry, index);
                fixture.push(route);
                fixture.assert_app_bar_reads(entry.sub_title(index));
                fixture.go_back();
                fixture.assert_app_bar_reads(entry.title());
                fixture.go_back();
            }
        }
        assert!(!fixture.can_pop(), "every sub-page came back off");
    });
}

/// The index's drawer lists the entries too, and a tile closes it before it pushes.
#[test]
fn the_index_drawer_opens_an_entry() {
    on_each_platform(|fixture| {
        let drawer_button = fixture.last::<DrawerButton>();
        fixture.tap(drawer_button);
        assert!(
            fixture.has::<DrawerHeader>(),
            "the drawer button opened the drawer"
        );

        let drawer = fixture.last::<Drawer>();
        let tile = fixture.tile_in(drawer, Entry::Lists.title());
        fixture.tap(tile);
        fixture.assert_app_bar_reads(Entry::Lists.title());
        fixture.assert_top_route_settled();
        fixture.go_back();
        assert!(!fixture.can_pop(), "the Lists page came back off");
        assert!(
            !fixture.has::<DrawerHeader>(),
            "the drawer closed before the page was pushed"
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

        let app_bar = fixture.last::<AppBar>();
        let toggle = fixture.last_in::<IconButton>(app_bar);
        fixture.tap(toggle);
        assert_eq!(fixture.brightness(), Brightness::Dark);

        let app_bar = fixture.last::<AppBar>();
        let toggle = fixture.last_in::<IconButton>(app_bar);
        fixture.tap(toggle);
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
        assert!(
            fixture.shows("Last pressed: \u{2014}"),
            "nothing pressed yet; saw {:?}",
            fixture.texts()
        );

        fixture.tap_text("Compose");
        assert!(
            fixture.shows("Last pressed: Scaffold FAB"),
            "the floating action button's on_pressed reached the page; saw {:?}",
            fixture.texts()
        );

        fixture.go_back();
        assert!(!fixture.can_pop());
    });
}

/// The end drawer opens from its button, and a tile closes it. The button is the last of the
/// app bar's actions; the drawer button before it is
/// [`the_drawer_opens_from_its_app_bar_action`].
#[test]
fn the_end_drawer_opens_from_the_app_bar_and_closes_from_a_tile() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Drawers);

        let end_drawer_button = fixture.last::<EndDrawerButton>();
        fixture.tap(end_drawer_button);
        assert!(
            fixture.shows(END_DRAWER_HEADER),
            "the end drawer is open; saw {:?}",
            fixture.texts()
        );
        fixture.tap_text("Starred only");
        assert!(
            !fixture.shows(END_DRAWER_HEADER),
            "the tile closed the end drawer; saw {:?}",
            fixture.texts()
        );
        assert!(fixture.shows("Chose Starred only"));

        fixture.go_back();
        assert!(!fixture.can_pop());
    });
}

/// A swipe in from the start edge opens the drawer on Android. On macOS a `MaterialPageRoute`
/// slides as Cupertino's does, and the same swipe is its back gesture: the detector for it
/// sits over the page, so its recognizer joins the arena before the drawer's and wins it.
#[test]
fn the_start_edge_swipe_opens_the_drawer_or_goes_back() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Drawers);
        let platform = fixture.cell.borrow().platform().target_platform();

        fixture.drag(Offset::new(4.0, VIEW[1] / 2.0), Offset::new(280.0, 0.0));
        if platform == TargetPlatform::MacOS {
            assert!(!fixture.can_pop(), "the swipe went back to the index");
            return;
        }
        assert!(
            fixture.shows(DRAWER_HEADER),
            "the edge swipe opened the drawer; saw {:?}",
            fixture.texts()
        );
        fixture.tap_text("Outbox");
        assert!(
            !fixture.shows(DRAWER_HEADER),
            "the tile closed the drawer; saw {:?}",
            fixture.texts()
        );
        assert!(fixture.shows("Chose Outbox"));

        fixture.go_back();
        assert!(!fixture.can_pop());
    });
}

/// The close-icon snack bar, the last button of its row, and its close icon, the last button
/// of the snack bar.
#[test]
fn a_snack_bar_shows_and_closes_from_its_icon() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Messages);

        fixture.tap_text("With close icon");
        assert!(
            fixture.shows("A snack bar with a close icon"),
            "the snack bar is up; saw {:?}",
            fixture.texts()
        );
        let snack_bar = fixture.last::<SnackBar>();
        let close = fixture.last_in::<IconButton>(snack_bar);
        fixture.tap(close);
        assert!(
            !fixture.shows("A snack bar with a close icon"),
            "the close icon hid the snack bar; saw {:?}",
            fixture.texts()
        );

        fixture.go_back();
        assert!(!fixture.can_pop());
    });
}

/// The persistent sheet, the last button of its row, closed by the sheet's one button.
#[test]
fn a_persistent_bottom_sheet_opens_and_closes() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Messages);

        fixture.tap_text("Persistent sheet");
        assert!(
            fixture.shows(PERSISTENT_SHEET_TEXT),
            "the persistent sheet is up; saw {:?}",
            fixture.texts()
        );
        fixture.tap_text("Close persistent sheet");
        assert!(
            !fixture.shows(PERSISTENT_SHEET_TEXT),
            "the sheet's button closed it; saw {:?}",
            fixture.texts()
        );

        fixture.go_back();
        assert!(!fixture.can_pop());
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
        assert!(!fixture.can_pop());
    });
}

/// How much of the view the top page's `SliverAppBar` covers.
fn app_bar_extent(fixture: &mut Fixture) -> f64 {
    let app_bar = fixture.last::<SliverAppBar>();
    let app = fixture.cell.borrow();
    app_bar
        .render_object(&app)
        .and_then(AnyRenderObject::as_sliver)
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
        assert!(!fixture.can_pop());
    });
}

#[test]
fn tapping_a_tile_selects_it() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Lists);
        assert!(
            fixture.shows("Selected: Inbox"),
            "the first mailbox starts selected; saw {:?}",
            fixture.texts()
        );

        fixture.tap_text("Starred");
        assert!(
            fixture.shows("Selected: Starred"),
            "the tile's on_tap reached the page; saw {:?}",
            fixture.texts()
        );
        let selected: Vec<bool> = fixture
            .elements()
            .into_iter()
            .filter_map(|element| {
                let app = fixture.cell.borrow();
                let widget = element.widget(&app);
                let tile = downcast_widget::<ListTile>(&**widget)?;
                Some(tile.selected)
            })
            .collect();
        assert_eq!(
            selected.iter().filter(|selected| **selected).count(),
            1,
            "one tile is selected"
        );

        fixture.go_back();
        assert!(!fixture.can_pop());
    });
}

/// The banner's button is alone in its row, and its Dismiss action alone in the banner's.
#[test]
fn a_banner_shows_and_dismisses() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Messages);

        fixture.tap_text("Show banner");
        assert!(
            fixture.shows(BANNER_TEXT),
            "the banner is up; saw {:?}",
            fixture.texts()
        );
        fixture.tap_text("Dismiss");
        assert!(
            !fixture.shows(BANNER_TEXT),
            "its action hid the banner; saw {:?}",
            fixture.texts()
        );

        fixture.go_back();
        assert!(!fixture.can_pop());
    });
}

/// Taps the common button reading `label` as `W`, and answers what the page's last-pressed
/// line reads afterwards, if it does not read `expected`.
fn press_common_button<W: 'static>(
    fixture: &mut Fixture,
    label: &str,
    expected: &str,
) -> Option<String> {
    let button = fixture.button_reading::<W>(label);
    fixture.tap(button);
    let pressed = format!("Last pressed: {expected}");
    if fixture.shows(&pressed) {
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
#[ignore = "the Filled and Tonal rows each have a FilledButton reading \"Icon\", and `button_reading` finds the first, so \"Tonal icon\" taps Filled's"]
fn each_common_button_reports_its_press() {
    on_each_platform(|fixture| {
        fixture.open_from_index(Entry::Buttons);
        let missed: Vec<String> = [
            press_common_button::<ElevatedButton>(fixture, "Elevated", "Elevated"),
            press_common_button::<ElevatedButton>(fixture, "Icon", "Elevated icon"),
            press_common_button::<FilledButton>(fixture, "Filled", "Filled"),
            press_common_button::<FilledButton>(fixture, "Icon", "Filled icon"),
            press_common_button::<FilledButton>(fixture, "Tonal", "Tonal"),
            press_common_button::<FilledButton>(fixture, "Icon", "Tonal icon"),
            press_common_button::<OutlinedButton>(fixture, "Outlined", "Outlined"),
            press_common_button::<OutlinedButton>(fixture, "Icon", "Outlined icon"),
            press_common_button::<TextButton>(fixture, "Text", "Text"),
            press_common_button::<TextButton>(fixture, "Icon", "Text icon"),
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
        let mut missed = Vec::new();
        for variant in ["Standard", "Filled", "Tonal"] {
            let toggle = fixture.icon_button_with_tooltip(variant);
            fixture.tap(toggle);
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
        let drawer_button = fixture.last::<DrawerButton>();
        fixture.tap(drawer_button);
        assert!(
            fixture.shows(DRAWER_HEADER),
            "the drawer is open; saw {:?}",
            fixture.texts()
        );
    });
}

/// Taps the button reading `label` on the Messages page and checks `shown` comes up.
fn assert_messages_button_shows(fixture: &mut Fixture, label: &str, shown: &str) {
    fixture.open_from_index(Entry::Messages);
    fixture.tap_text(label);
    assert!(
        fixture.shows(shown),
        "{label:?} shows {shown:?}; saw {:?}",
        fixture.texts()
    );
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

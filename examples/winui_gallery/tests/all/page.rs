//! Page's background under the system's bars, and its content clear of them and of the keyboard.

use std::{cell::RefCell, rc::Rc};
use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::EdgeInsets;
use waft_widgets::*;
use waft_winui::{Brush, SizeObserver};
use waft_winui_test_support::{Fixture, mount};

const VIEW: [u32; 2] = [100, 200];
const BACKGROUND: Color = Color::from_argb(255, 0, 0, 255);
const CONTENT: Color = Color::from_argb(255, 0, 255, 0);

/// What the page's content saw: its size, and the padding and keyboard inset its `MediaQuery` still reported.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seen {
    size: Size,
    padding: EdgeInsets,
    keyboard: f64,
}

/// The view's metrics as a phone's host reports them: the bars as `view_padding`, the keyboard as `view_insets`, and `padding` the bars the keyboard leaves uncovered.
fn phone(bars: EdgeInsets, keyboard: f64) -> MediaQueryData {
    MediaQueryData::new()
        .size(Size::new(VIEW[0] as f64, VIEW[1] as f64))
        .view_padding(bars)
        .view_insets(EdgeInsets::from_ltrb(0.0, 0.0, 0.0, keyboard))
        .padding(EdgeInsets::from_ltrb(
            bars.left,
            bars.top,
            bars.right,
            (bars.bottom - keyboard).max(0.0),
        ))
}

impl Default for Seen {
    fn default() -> Self {
        Seen {
            size: Size::ZERO,
            padding: EdgeInsets::ZERO,
            keyboard: 0.0,
        }
    }
}

/// Content filling what the page gives it, reporting what it saw.
fn reporting_content(seen: Rc<RefCell<Seen>>) -> WidgetRef {
    let sized = seen.clone();
    SizeObserver::new(
        Builder::new(move |app, context| {
            let media = MediaQuery::of(app, context);
            let mut seen = seen.borrow_mut();
            seen.padding = media.padding;
            seen.keyboard = media.view_insets.bottom;
            ColoredBox::new(CONTENT)
                .child(SizedBox::expand())
                .into_widget()
        }),
        Rc::new(move |_: &mut App, _, new| sized.borrow_mut().size = new),
    )
    .into_widget()
}

/// Mounts `page_around(content)` under `metrics` and answers what the content saw.
fn mount_page(
    metrics: MediaQueryData,
    page_around: impl Fn(WidgetRef) -> WidgetRef + 'static,
) -> (Fixture, Seen) {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let reported = seen.clone();
    let fixture = Fixture::with_root(
        VIEW,
        mount(move |_| {
            MediaQuery::new(metrics, page_around(reporting_content(reported))).into_widget()
        }),
    );
    let seen = *seen.borrow();
    (fixture, seen)
}

fn page(content: WidgetRef) -> WidgetRef {
    waft_winui::Page::new()
        .background(Brush::Solid(BACKGROUND))
        .content(content)
        .into_widget()
}

fn pixel(fixture: &Fixture, x: usize, y: usize) -> [u8; 4] {
    let pixels = fixture.view.pixels.borrow();
    pixels[(y * VIEW[0] as usize + x) * 4..][..4]
        .try_into()
        .unwrap()
}

#[test]
fn content_stays_inside_the_bars_while_the_background_reaches_under_them() {
    let (fixture, seen) = mount_page(
        phone(EdgeInsets::from_ltrb(0.0, 24.0, 0.0, 16.0), 0.0),
        page,
    );
    assert_eq!(seen.size, Size::new(100.0, 160.0));
    assert_eq!(
        pixel(&fixture, 50, 10),
        [0, 0, 255, 255],
        "background under the status bar"
    );
    assert_eq!(pixel(&fixture, 50, 100), [0, 255, 0, 255], "content");
    assert_eq!(
        pixel(&fixture, 50, 190),
        [0, 0, 255, 255],
        "background under the navigation bar"
    );
}

#[test]
fn the_keyboard_shrinks_the_content_to_above_it() {
    // With the keyboard up the host reports no bottom padding: the keyboard covers the navigation bar.
    let (_, seen) = mount_page(
        phone(EdgeInsets::from_ltrb(0.0, 24.0, 0.0, 16.0), 80.0),
        page,
    );
    assert_eq!(seen.size, Size::new(100.0, 96.0));
}

#[test]
fn the_content_sees_neither_the_bars_nor_the_keyboard() {
    let (_, seen) = mount_page(
        phone(EdgeInsets::from_ltrb(8.0, 24.0, 8.0, 16.0), 80.0),
        page,
    );
    assert_eq!(seen.padding, EdgeInsets::ZERO);
    assert_eq!(seen.keyboard, 0.0);
}

#[test]
fn a_page_inside_a_page_adds_no_inset() {
    let (_, seen) = mount_page(
        phone(EdgeInsets::from_ltrb(0.0, 24.0, 0.0, 16.0), 80.0),
        |content| page(page(content)),
    );
    assert_eq!(seen.size, Size::new(100.0, 96.0));
}

#[test]
fn without_bars_the_content_fills_the_page() {
    let (_, seen) = mount_page(phone(EdgeInsets::ZERO, 0.0), page);
    assert_eq!(seen.size, Size::new(100.0, 200.0));
}

#[test]
fn extending_into_the_bars_hands_them_to_the_content() {
    let (_, seen) = mount_page(
        phone(EdgeInsets::from_ltrb(0.0, 24.0, 0.0, 16.0), 0.0),
        |content| {
            waft_winui::Page::new()
                .background(Brush::Solid(BACKGROUND))
                .extends_content_into_system_bars(true)
                .content(content)
                .into_widget()
        },
    );
    assert_eq!(
        seen.size,
        Size::new(100.0, 200.0),
        "the content reaches the view's edges"
    );
    assert_eq!(seen.padding, EdgeInsets::from_ltrb(0.0, 24.0, 0.0, 16.0));
}

/// A navigation on a page extending into a phone's bars, `width` wide, in `theme`.
fn navigation_under_bars(width: u32, bars: EdgeInsets) -> Fixture {
    use waft_winui::*;
    Fixture::with_root([width, 400], move |app| {
        winui_gallery::install_fonts(app);
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |app, context, _| {
                    let media = MediaQuery::of(app, context);
                    let navigation = NavigationView::new(
                        vec![NavigationViewItem::text("home", "Home")],
                        Some("home".to_owned()),
                        |_, _| {},
                        Text::new("Body"),
                    )
                    .header(Text::new("Title"));
                    MediaQuery::new(
                        media.copy_with().padding(bars).view_padding(bars),
                        ThemeScope::new(
                            Theme::Light,
                            Overlay::wrap(
                                Page::new()
                                    .background(Brush::Solid(BACKGROUND))
                                    .extends_content_into_system_bars(true)
                                    .content(navigation),
                            ),
                        ),
                    )
                    .into_widget()
                })
                .into_widget(),
        );
    })
}

#[test]
fn a_navigation_reaches_the_edges_and_keeps_its_controls_clear_of_the_bars() {
    let bars = EdgeInsets::from_ltrb(0.0, 40.0, 0.0, 30.0);
    // Minimal: the content's layer is the whole view.
    let f = navigation_under_bars(400, bars);
    let title = f.find("Title");
    assert!(
        title.dy() > 40.0,
        "the header is below the status bar: {title:?}"
    );
    let pixels = f.view.pixels.borrow();
    let pixel =
        |x: usize, y: usize| -> [u8; 4] { pixels[(y * 400 + x) * 4..][..4].try_into().unwrap() };
    assert_ne!(
        pixel(200, 10),
        [0, 0, 255, 255],
        "no page background under the status bar"
    );
    assert_ne!(
        pixel(200, 390),
        [0, 0, 255, 255],
        "nor under the gesture bar"
    );
    assert_eq!(
        pixel(200, 10),
        pixel(200, 200),
        "the content's layer reaches the top"
    );
    drop(pixels);
    // Expanded: the rail and the content card reach the bottom.
    let f = navigation_under_bars(1100, bars);
    let home = f.find("Home");
    assert!(
        home.dy() > 40.0,
        "the pane's items are below the status bar: {home:?}"
    );
    let pixels = f.view.pixels.borrow();
    let at =
        |x: usize, y: usize| -> [u8; 4] { pixels[(y * 1100 + x) * 4..][..4].try_into().unwrap() };
    assert_eq!(
        at(20, 395),
        at(20, 200),
        "the pane looks the same down to the bottom"
    );
    assert_ne!(
        at(800, 395),
        [0, 0, 255, 255],
        "the content reaches the bottom"
    );
}

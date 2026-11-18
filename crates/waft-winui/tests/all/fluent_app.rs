//! `FluentApp`'s theme, accent, icon font and page route.

use std::cell::Cell;
use std::rc::Rc;

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::PaintingBinding;
use waft_widgets::testing::WidgetTester;
use waft_widgets::*;
use waft_winui::{AccentPalette, FluentApp, FluentTheme, FluentThemeMode, Theme, ThemeResources};

/// A fixed accent, so the test can tell the theme's palette from WinUI's default blue.
const ACCENT: AccentPalette = AccentPalette {
    base: Color::from_argb(255, 1, 2, 3),
    light1: Color::from_argb(255, 4, 5, 6),
    light2: Color::from_argb(255, 7, 8, 9),
    light3: Color::from_argb(255, 10, 11, 12),
    dark1: Color::from_argb(255, 13, 14, 15),
    dark2: Color::from_argb(255, 16, 17, 18),
    dark3: Color::from_argb(255, 19, 20, 21),
};

/// A page that records the theme resources around it and is built.
#[derive(Debug)]
struct Probe {
    seen: Rc<Cell<Option<(Theme, AccentPalette)>>>,
    built: Rc<Cell<bool>>,
}

impl StatelessWidget for Probe {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        self.seen.set(Some((resources.theme, resources.accent)));
        self.built.set(true);
        SizedBox::shrink().into_widget()
    }
}

fn probe() -> (
    Probe,
    Rc<Cell<Option<(Theme, AccentPalette)>>>,
    Rc<Cell<bool>>,
) {
    let seen = Rc::new(Cell::new(None));
    let built = Rc::new(Cell::new(false));
    (
        Probe {
            seen: Rc::clone(&seen),
            built: Rc::clone(&built),
        },
        seen,
        built,
    )
}

#[test]
fn a_fluent_app_shows_the_theme_and_accent_it_was_given() {
    let mut tester = WidgetTester::new();
    let (page, seen, built) = probe();
    tester.pump_widget(
        FluentApp::new()
            .theme(FluentTheme::new(Theme::Dark, ACCENT))
            .home(page)
            .into_widget(),
    );
    tester.pump_and_settle();
    assert!(built.get(), "the home page was built");
    assert_eq!(seen.get(), Some((Theme::Dark, ACCENT)));
}

#[test]
fn the_dark_theme_is_shown_when_the_mode_says_so() {
    let mut tester = WidgetTester::new();
    let (page, seen, _built) = probe();
    tester.pump_widget(
        FluentApp::new()
            .theme(FluentTheme::new(Theme::Light, ACCENT))
            .dark_theme(FluentTheme::new(Theme::Dark, ACCENT))
            .theme_mode(FluentThemeMode::Dark)
            .home(page)
            .into_widget(),
    );
    tester.pump_and_settle();
    assert_eq!(seen.get(), Some((Theme::Dark, ACCENT)));
}

#[test]
fn a_fluent_app_builds_routes_without_the_app_naming_a_page_route() {
    let mut tester = WidgetTester::new();
    let built = Rc::new(Cell::new(false));
    let heard = Rc::clone(&built);
    tester.pump_widget(
        FluentApp::new()
            .routes([(
                String::from("/"),
                Rc::new(move |_app: &mut App, _context: BuildContext| {
                    heard.set(true);
                    SizedBox::shrink().into_widget()
                }) as WidgetBuilder,
            )])
            .into_widget(),
    );
    tester.pump_and_settle();
    assert!(built.get(), "the \"/\" route was built");
}

#[test]
fn a_fluent_app_installs_the_icon_font_when_it_mounts() {
    let mut tester = WidgetTester::new();
    let (page, _seen, _built) = probe();
    tester.pump_widget(FluentApp::new().home(page).into_widget());
    tester.pump_and_settle();
    let mut app = tester.cell.borrow_mut();
    let painting = PaintingBinding::instance(&mut app);
    assert!(painting.has_fonts(&app), "the icon font was registered");
}

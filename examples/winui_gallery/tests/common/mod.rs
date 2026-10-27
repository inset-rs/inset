//! Gallery setup and page navigation layered on reusable test support.
#![allow(dead_code, unused_imports)]

pub use inset_winui_test_support::{Fixture, mount};

use inset_embedder::Size;
use inset_foundation::App;
use inset_widgets::*;
use inset_winui::{AccentPalette, Theme, ThemeScope};
use std::rc::Rc;

/// Gallery-only entry points; the shared fixture has no dependency on gallery code.
pub trait GalleryFixtureExt: Sized {
    /// Mounts the gallery's initial page.
    fn new(size: [u32; 2]) -> Self;

    /// Mounts the gallery directly on one feature.
    fn for_feature(size: [u32; 2], feature: winui_gallery::Feature) -> Self;

    /// Navigates through the gallery's visible navigation item.
    fn navigate(&mut self, feature: winui_gallery::Feature);
}

impl GalleryFixtureExt for Fixture {
    fn new(size: [u32; 2]) -> Self {
        Self::with_root(size, winui_gallery::run)
    }

    fn for_feature(size: [u32; 2], feature: winui_gallery::Feature) -> Self {
        Self::with_root(size, move |app| winui_gallery::run_feature(app, feature))
    }

    fn navigate(&mut self, feature: winui_gallery::Feature) {
        self.ensure_visible(feature.title());
        self.tap(feature.title());
        self.pump();
    }
}

/// The size `control` takes when `Center` offers it the whole of a `size` view, more room than
/// it needs, in the light theme with the gallery's fonts.
pub fn size_under_center(
    size: [u32; 2],
    control: impl FnOnce(&mut App) -> WidgetRef + 'static,
) -> Size {
    let key = Rc::new(GlobalKey::new());
    let control_key = key.clone();
    let fixture = Fixture::with_root(
        size,
        mount(move |app| {
            winui_gallery::install_fonts(app);
            let control = KeyedSubtree::new(control(app)).key(control_key);
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .page_route_builder(|app, settings, builder| {
                    let route = PageRouteBuilder::new(
                        app,
                        Rc::new(move |app, context, _, _| builder(app, context)),
                    )
                    .settings(app, RouteSettingsRef::Settings(settings.clone()));
                    PageRoute::as_page_route(route)
                })
                .home(ThemeScope::new(Theme::Light, Center::new().child(control)))
                .into_widget()
        }),
    );
    let mut app = fixture.cell.borrow_mut();
    let context = key.current_context(&mut app).unwrap();
    let object = context.find_render_object(&app).unwrap().as_box().unwrap();
    object.size(&app)
}

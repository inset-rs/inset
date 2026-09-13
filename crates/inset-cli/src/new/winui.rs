//! The WinUI template: a themed page with one button, sized for a desktop window.

use super::Kit;

pub const KIT: Kit = Kit {
    dependencies: "inset-winui = \"0.1\"\n",
    lib_rs: r#"// StatefulWidget::State takes `self: Handle<Self>`.
#![feature(arbitrary_self_types)]

use std::rc::Rc;

use inset::{
    App, BuildContext, Center, ColoredBox, IntoWidget, Listener, PageRoute, PageRouteBuilder,
    RouteSettingsRef, StatelessWidget, WidgetRef, WidgetsApp, run_app,
};
use inset_winui::{AccentPalette, Button, Theme, ThemeResources, ThemeScope, install_icon_font};

#[inset::main(size = [1080.0, 780.0])]
fn main(app: &mut App) {
    // The kit's icon glyphs, registered once before the first frame.
    install_icon_font(app);
    run_app(
        app,
        WidgetsApp::new(AccentPalette::default().base)
            .page_route_builder(|app, settings, builder| {
                let route = PageRouteBuilder::new(
                    app,
                    Rc::new(move |app, context, _, _| builder(app, context)),
                )
                .settings(app, RouteSettingsRef::Settings(settings.clone()));
                PageRoute::as_page_route(route)
            })
            .home(Home)
            .into_widget(),
    );
}

#[derive(Debug)]
struct Home;

impl StatelessWidget for Home {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let theme = Theme::Light;
        let resources = ThemeResources::new(theme, AccentPalette::default());
        let click = Listener::new(|_app| {});
        ThemeScope::new(
            theme,
            ColoredBox::new(resources.common.solid_background_fill_color_base)
                .child(Center::new().child(Button::text("Hello, Inset", click))),
        )
        .into_widget()
    }
}
"#,
};

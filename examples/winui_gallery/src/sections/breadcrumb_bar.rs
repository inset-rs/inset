//! BreadcrumbBar navigation, overflow, live collection updates and custom item content.

use crate::{column, example, row, section};
use inset_embedder::TextDirection;
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the feature page.
pub fn page(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("BreadcrumbBar", resources, BreadcrumbDemo.into_widget())
}

/// The application owns its current path.
#[derive(Debug)]
struct BreadcrumbDemo;

/// Settings and navigation result used by the examples.
struct BreadcrumbDemoState {
    /// Native widget state.
    state: StateData<BreadcrumbDemo>,

    /// Number of entries in the displayed path.
    count: usize,

    /// Narrow layout exposes the ellipsis flyout.
    narrow: bool,

    /// Direction of the breadcrumb examples.
    rtl: bool,

    /// Most recently invoked original index.
    last: String,
}

impl StatefulWidget for BreadcrumbDemo {
    type State = BreadcrumbDemoState;

    fn create_state(&self) -> Self::State {
        BreadcrumbDemoState {
            state: StateData::new(),
            count: 5,
            narrow: false,
            rtl: false,
            last: "None".into(),
        }
    }
}

impl State for BreadcrumbDemoState {
    type Widget = BreadcrumbDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let names = ["Home", "Documents", "Projects", "Inset", "Examples"];
        let items = names
            .iter()
            .take(app.get(self).count)
            .enumerate()
            .map(|(index, name)| BreadcrumbBarItem::text(index.to_string(), *name))
            .collect();
        let bar = BreadcrumbBar::new(items, move |app, args| {
            self.set_state(app, |state| {
                state.count = args.index + 1;
                state.last = format!("{} (index {})", names[args.index], args.index);
            });
        });
        let custom = BreadcrumbBar::new(
            vec![
                BreadcrumbBarItem::new(
                    "workspace",
                    row(
                        vec![
                            FontIcon::symbol(FluentSymbol::Apps)
                                .font_size(16.0)
                                .into_widget(),
                            Text::new("Workspace").into_widget(),
                        ],
                        6.0,
                    ),
                ),
                BreadcrumbBarItem::text("disabled", "Unavailable").is_enabled(false),
                BreadcrumbBarItem::text("current", "Current page"),
            ],
            move |app, args| self.set_state(app, |state| state.last = args.item.id),
        );
        let direction = if app.get(self).rtl {
            TextDirection::Rtl
        } else {
            TextDirection::Ltr
        };
        column(
            vec![
                example(
                    "Navigate a path",
                    "Choose an earlier destination to shorten the path. Tab enters the bar; Left and Right move between visible items.",
                    &resources,
                    Directionality::new(
                        direction,
                        SizedBox::new()
                            .width(if app.get(self).narrow { 180.0 } else { 640.0 })
                            .child(bar),
                    )
                    .into_widget(),
                ),
                example(
                    "Overflow and direction",
                    "The ellipsis lists hidden ancestors from nearest to farthest.",
                    &resources,
                    row(
                        vec![
                            Button::text(
                                if app.get(self).narrow {
                                    "Use wide layout"
                                } else {
                                    "Use narrow layout"
                                },
                                Listener::new(move |app| {
                                    self.set_state(app, |s| s.narrow = !s.narrow)
                                }),
                            )
                            .into_widget(),
                            Button::text(
                                "Reset path",
                                Listener::new(move |app| self.set_state(app, |s| s.count = 5)),
                            )
                            .into_widget(),
                            Button::text(
                                if app.get(self).rtl {
                                    "Direction: RTL"
                                } else {
                                    "Direction: LTR"
                                },
                                Listener::new(move |app| self.set_state(app, |s| s.rtl = !s.rtl)),
                            )
                            .into_widget(),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Custom content",
                    "Item content can include icons. Disabled destinations are skipped by arrow navigation.",
                    &resources,
                    Directionality::new(direction, custom).into_widget(),
                ),
                Text::new(format!("Invoked: {}", app.get(self).last)).into_widget(),
            ],
            16.0,
        )
    }
}

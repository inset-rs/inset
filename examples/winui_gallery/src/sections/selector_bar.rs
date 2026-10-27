//! SelectorBar: switching between a few views of one page, with text and icons, a disabled item,
//! and right-to-left.

use crate::{column, example, label, row, section};
use inset_embedder::TextDirection;
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the feature page.
pub fn page(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("SelectorBar", resources, SelectorBarDemo.into_widget())
}

/// The application owns each bar's selection.
#[derive(Debug)]
struct SelectorBarDemo;

/// Each example's selected item and the direction.
struct SelectorBarDemoState {
    /// Native widget state.
    state: StateData<SelectorBarDemo>,

    /// The text bar's selection.
    files: Option<usize>,

    /// The icon bar's selection.
    views: Option<usize>,

    /// Direction of the examples.
    rtl: bool,
}

impl StatefulWidget for SelectorBarDemo {
    type State = SelectorBarDemoState;

    fn create_state(&self) -> Self::State {
        SelectorBarDemoState {
            state: StateData::new(),
            files: Some(0),
            views: None,
            rtl: false,
        }
    }
}

/// The names of the text bar's views.
const FILES: [&str; 4] = ["Recent", "Shared", "Favorites", "Archived"];

impl State for SelectorBarDemoState {
    type Widget = SelectorBarDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let state = app.get(self);
        let (files, views, rtl) = (state.files, state.views, state.rtl);
        let files_bar = SelectorBar::new(
            FILES
                .iter()
                .map(|name| SelectorBarItem::new(*name, *name))
                .collect(),
            files,
            move |app, picked| self.set_state(app, |state| state.files = picked),
        );
        let views_bar = SelectorBar::new(
            vec![
                SelectorBarItem::new("find", "Find").icon(FontIcon::symbol(FluentSymbol::Search)),
                SelectorBarItem::new("layers", "Layers")
                    .icon(FontIcon::symbol(FluentSymbol::Layer)),
                SelectorBarItem::new("settings", "Settings")
                    .icon(FontIcon::symbol(FluentSymbol::Settings))
                    .is_enabled(false),
            ],
            views,
            move |app, picked| self.set_state(app, |state| state.views = picked),
        );
        let shown = |picked: Option<usize>, names: &[&str]| {
            picked.map_or("Nothing selected".to_owned(), |index| {
                format!("Showing {}", names[index])
            })
        };
        let direction = if rtl {
            TextDirection::Rtl
        } else {
            TextDirection::Ltr
        };
        let secondary = resources.text_fill_color_secondary;
        column(
            vec![
                example(
                    "Switch views",
                    "Click a view, or Tab into the bar and use Left, Right, Home and End; selection follows the focus.",
                    &resources,
                    column(
                        vec![
                            Directionality::new(direction, files_bar).into_widget(),
                            label(shown(files, &FILES), TextBlockStyle::Caption, secondary),
                        ],
                        8.0,
                    ),
                ),
                example(
                    "Icons, a disabled view and no selection",
                    "The bar starts with nothing selected; focusing it selects the first view it can. Settings is disabled.",
                    &resources,
                    column(
                        vec![
                            Directionality::new(direction, views_bar).into_widget(),
                            label(
                                shown(views, &["Find", "Layers", "Settings"]),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        8.0,
                    ),
                ),
                row(
                    vec![
                        Button::text(
                            if rtl {
                                "Direction: RTL"
                            } else {
                                "Direction: LTR"
                            },
                            Listener::new(move |app| self.set_state(app, |s| s.rtl = !s.rtl)),
                        )
                        .into_widget(),
                        Button::text(
                            "Clear selections",
                            Listener::new(move |app| {
                                self.set_state(app, |s| {
                                    s.files = None;
                                    s.views = None;
                                })
                            }),
                        )
                        .into_widget(),
                    ],
                    12.0,
                ),
            ],
            24.0,
        )
    }
}

//! Adaptive left/top navigation, nested items, footer actions and back requests.

use crate::{column, example, example_row, label, section};
use inset_foundation::{App, Handle, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_widgets::*;
use inset_winui::*;

/// Builds the interactive NavigationView gallery section.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "NavigationView",
        resources,
        example(
            "Adaptive navigation and pane chrome",
            "Switch pane layouts, or enable the narrow preview to use automatic adaptation. Custom pane chrome replaces the default workspace title.",
            resources,
            NavigationDemo.into_widget(),
        ),
    )
}

/// Owns the selected page while NavigationView owns its expansion and adaptive pane state.
#[derive(Debug)]
struct NavigationDemo;

/// Interactive pane configuration and owner-managed selection.
struct NavigationDemoState {
    /// Native widget lifecycle.
    state: StateData<NavigationDemo>,

    /// Stable identity of the selected destination.
    selected: Option<String>,

    /// Current pane presentation mode.
    mode: NavigationViewPaneDisplayMode,

    /// Whether footer menu items are present.
    footer: bool,

    /// Whether the back action is available.
    back: bool,

    /// Most recent navigation event.
    event: String,

    /// Whether the optional pane slots are populated.
    chrome: bool,

    /// Constrains the preview below the compact adaptive threshold.
    narrow: bool,
}

impl StatefulWidget for NavigationDemo {
    type State = NavigationDemoState;

    fn create_state(&self) -> Self::State {
        Self::State {
            state: StateData::new(),
            selected: Some("overview".into()),
            mode: NavigationViewPaneDisplayMode::Left,
            footer: true,
            back: true,
            event: "Select a page or expand Library".into(),
            chrome: false,
            narrow: false,
        }
    }
}

impl State for NavigationDemoState {
    type Widget = NavigationDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let text = resources.text_fill_color_primary;
        let s = app.get(self);
        let (mode, footer, back, selected, event) = (
            s.mode,
            s.footer,
            s.back,
            s.selected.clone(),
            s.event.clone(),
        );
        let (chrome, narrow) = (s.chrome, s.narrow);
        let mode_name = match mode {
            NavigationViewPaneDisplayMode::Auto => "Auto",
            NavigationViewPaneDisplayMode::Left => "Left",
            NavigationViewPaneDisplayMode::LeftCompact => "LeftCompact",
            NavigationViewPaneDisplayMode::LeftMinimal => "LeftMinimal",
            NavigationViewPaneDisplayMode::Top => "Top",
        };
        let controls = example_row(
            vec![
                Button::text(
                    format!("Mode: {mode_name}"),
                    Listener::new(move |app| {
                        self.set_state(app, |s| {
                            s.mode = match s.mode {
                                NavigationViewPaneDisplayMode::Auto => {
                                    NavigationViewPaneDisplayMode::Left
                                }
                                NavigationViewPaneDisplayMode::Left => {
                                    NavigationViewPaneDisplayMode::LeftCompact
                                }
                                NavigationViewPaneDisplayMode::LeftCompact => {
                                    NavigationViewPaneDisplayMode::LeftMinimal
                                }
                                NavigationViewPaneDisplayMode::LeftMinimal => {
                                    NavigationViewPaneDisplayMode::Top
                                }
                                NavigationViewPaneDisplayMode::Top => {
                                    NavigationViewPaneDisplayMode::Auto
                                }
                            };
                        })
                    }),
                )
                .into_widget(),
                CheckBox::new(Some(footer), move |app, value| {
                    self.set_state(app, |s| s.footer = value.unwrap_or(false))
                })
                .content(Text::new("Footer items"))
                .into_widget(),
                CheckBox::new(Some(back), move |app, value| {
                    self.set_state(app, |s| s.back = value.unwrap_or(false))
                })
                .content(Text::new("Back button"))
                .into_widget(),
                CheckBox::new(Some(chrome), move |app, value| {
                    self.set_state(app, |s| s.chrome = value.unwrap_or(false))
                })
                .content(Text::new("Pane header and footer"))
                .into_widget(),
                CheckBox::new(Some(narrow), move |app, value| {
                    self.set_state(app, |s| {
                        s.narrow = value.unwrap_or(false);
                        if s.narrow {
                            s.mode = NavigationViewPaneDisplayMode::Auto;
                        }
                    })
                })
                .content(Text::new("Narrow navigation preview"))
                .into_widget(),
            ],
            12.0,
        );
        let items = vec![
            NavigationViewItem::text("overview", "Overview")
                .icon(FontIcon::symbol(FluentSymbol::Navigation)),
            NavigationViewItem::text("library", "Library")
                .icon(FontIcon::symbol(FluentSymbol::More))
                .menu_items([
                    NavigationViewItem::text("recent", "Recent"),
                    NavigationViewItem::text("collections", "Collections").menu_items([
                        NavigationViewItem::text("favorites", "Favorites"),
                        NavigationViewItem::text("archive", "Archive"),
                    ]),
                ]),
            NavigationViewItem::separator("divider"),
            NavigationViewItem::header("tools", "Tools"),
            NavigationViewItem::text("create", "Create new")
                .icon(FontIcon::symbol(FluentSymbol::Add))
                .selects_on_invoked(false),
            NavigationViewItem::text("unavailable", "Unavailable").is_enabled(false),
        ];
        let page = selected.clone().unwrap_or_else(|| "No selection".into());
        let content = Padding::new(EdgeInsetsGeometry::all(24.0)).child(label(
            "Arrow keys move between items. The chevron expands children without selecting the parent.",
            TextBlockStyle::Body,
            text,
        ));
        let mut navigation = NavigationView::new(
            items,
            selected,
            move |app, args| self.set_state(app, |s| s.selected = args.item.map(|item| item.id)),
            content,
        )
        .pane_display_mode(mode)
        .header(Text::new(page))
        .pane_title(if chrome { "" } else { "Workspace" })
        .is_back_button_visible(if back {
            NavigationViewBackButtonVisible::Visible
        } else {
            NavigationViewBackButtonVisible::Collapsed
        })
        .is_back_enabled(back)
        .item_invoked(move |app, args| {
            self.set_state(app, |s| s.event = format!("Invoked: {}", args.item.id))
        });
        if chrome {
            navigation = navigation
                .pane_header(
                    Padding::new(EdgeInsetsGeometry::all(12.0))
                        .child(Text::new("Personal workspace")),
                )
                .pane_custom_content(
                    Padding::new(EdgeInsetsGeometry::all(12.0))
                        .child(Text::new("Pinned destinations")),
                )
                .pane_footer(
                    Padding::new(EdgeInsetsGeometry::all(12.0))
                        .child(Text::new("Signed in locally")),
                );
        }
        if footer {
            navigation = navigation
                .footer_menu_items([NavigationViewItem::text("help", "Help")
                    .icon(FontIcon::symbol(FluentSymbol::More))]);
        }
        navigation.back_requested = Some(Listener::new(move |app| {
            self.set_state(app, |s| {
                s.selected = Some("overview".into());
                s.event = "Back requested".into();
            })
        }));
        let surface = SizedBox::new()
            .width(if narrow { 360.0 } else { 1000.0 })
            .height(400.0)
            .child(ColoredBox::new(resources.solid_background_fill_color_base).child(navigation));
        column(
            vec![
                controls,
                surface.into_widget(),
                label(event, TextBlockStyle::Caption, text),
            ],
            12.0,
        )
    }
}

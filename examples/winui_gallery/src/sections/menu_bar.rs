//! MenuBar examples for navigation, nested commands and live collection changes.

use crate::{column, example, row, section};
use inset_embedder::TextDirection;
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the menu-bar feature page.
pub fn page(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("MenuBar", resources, MenuBarDemo.into_widget())
}

/// Owns the example's checked values and header collection.
#[derive(Debug)]
struct MenuBarDemo;

/// Application state is independent of the menus' open/close lifetime.
struct MenuBarDemoState {
    /// Native widget state.
    state: StateData<MenuBarDemo>,

    /// Toggle item shown in the File menu.
    autosave: bool,

    /// Whether to include a third header.
    show_view: bool,

    /// Whether the Edit header accepts input.
    edit_enabled: bool,

    /// Direction applied only to the menu bar.
    rtl: bool,

    /// Most recently activated command.
    last: String,
}

impl StatefulWidget for MenuBarDemo {
    type State = MenuBarDemoState;

    fn create_state(&self) -> Self::State {
        MenuBarDemoState {
            state: StateData::new(),
            autosave: false,
            show_view: true,
            edit_enabled: true,
            rtl: false,
            last: "None".into(),
        }
    }
}

impl State for MenuBarDemoState {
    type Widget = MenuBarDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let action = |label: &'static str| {
            MenuFlyoutItem::new(
                label,
                Listener::new(move |app| {
                    self.set_state(app, |state| state.last = label.into());
                }),
            )
        };
        let mut headers = vec![
            MenuBarItem::new(
                "file",
                "File",
                vec![
                    action("New document").icon(FontIcon::symbol(FluentSymbol::Add)),
                    MenuFlyoutItem::toggle(
                        "Autosave",
                        app.get(self).autosave,
                        move |app, value| {
                            self.set_state(app, |state| state.autosave = value);
                        },
                    )
                    .prevent_dismiss_on_pointer(true),
                    MenuFlyoutItem::separator(),
                    MenuFlyoutItem::sub_item(
                        "Export",
                        vec![action("PDF document"), action("Plain text")],
                    ),
                ],
            ),
            MenuBarItem::new(
                "edit",
                "Edit",
                vec![action("Copy selection"), action("Paste selection")],
            )
            .is_enabled(app.get(self).edit_enabled),
        ];
        if app.get(self).show_view {
            headers.push(MenuBarItem::new(
                "view",
                "View",
                vec![action("Zoom in"), action("Zoom out")],
            ));
        }
        let bar = Directionality::new(
            if app.get(self).rtl {
                TextDirection::Rtl
            } else {
                TextDirection::Ltr
            },
            MenuBar::new(headers),
        );
        column(
            vec![
                example(
                    "Application commands",
                    "Open a menu, then move across headers with the pointer or arrow keys. Escape returns focus to the header.",
                    &resources,
                    bar.into_widget(),
                ),
                example(
                    "Collection and direction",
                    "These settings update the same menu bar while keeping its header identities.",
                    &resources,
                    row(
                        vec![
                            Button::text(
                                if app.get(self).show_view {
                                    "Hide View"
                                } else {
                                    "Show View"
                                },
                                Listener::new(move |app| {
                                    self.set_state(app, |s| s.show_view = !s.show_view)
                                }),
                            )
                            .into_widget(),
                            Button::text(
                                if app.get(self).edit_enabled {
                                    "Disable Edit"
                                } else {
                                    "Enable Edit"
                                },
                                Listener::new(move |app| {
                                    self.set_state(app, |s| s.edit_enabled = !s.edit_enabled)
                                }),
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
                    "Current state",
                    "The application owns checked values and command results.",
                    &resources,
                    column(
                        vec![
                            Text::new(format!("Autosave: {}", app.get(self).autosave))
                                .into_widget(),
                            Text::new(format!("Last command: {}", app.get(self).last))
                                .into_widget(),
                        ],
                        12.0,
                    ),
                ),
            ],
            24.0,
        )
    }
}

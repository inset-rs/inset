//! Split-button and menu examples with independent actions and retained menu settings.

use crate::{Feature, column, example, row, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds one feature page while giving it its own menu and application state.
pub fn page(feature: Feature, resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        feature.title(),
        resources,
        CommandDemo { feature }.into_widget(),
    )
}

/// Chooses the split, toggle-split or menu examples.
#[derive(Debug)]
struct CommandDemo {
    /// Catalog destination that owns this state.
    feature: Feature,
}

/// Owns the menu and the values changed by its commands.
struct CommandDemoState {
    /// Native widget state.
    state: StateData<CommandDemo>,

    /// Persistent menu shared by this page's opening buttons.
    menu: Option<Handle<MenuFlyout>>,

    /// Current toggle-split value.
    checked: bool,

    /// Current toggle menu value.
    autosave: bool,

    /// Current mutually exclusive layout choice.
    grid_view: bool,

    /// Number of primary activations.
    activations: usize,

    /// Most recently invoked menu action.
    last_action: String,
}

impl StatefulWidget for CommandDemo {
    type State = CommandDemoState;

    fn create_state(&self) -> Self::State {
        CommandDemoState {
            state: StateData::new(),
            menu: None,
            checked: false,
            autosave: false,
            grid_view: false,
            activations: 0,
            last_action: "None".into(),
        }
    }
}

impl CommandDemoState {
    /// Rebuilds owner-managed toggle and radio values while the menu remains open.
    fn refresh_menu(self: Handle<Self>, app: &mut App) {
        let items = self.items(app);
        app.get(self).menu.unwrap().items(app, items);
    }

    /// Commands, separators, disabled entries and nested collections in one menu.
    fn items(self: Handle<Self>, app: &App) -> Vec<MenuFlyoutItem> {
        let action = |text: &'static str| {
            MenuFlyoutItem::new(
                text,
                Listener::new(move |app| {
                    self.set_state(app, |state| state.last_action = text.into());
                }),
            )
        };
        vec![
            action("Save a copy").icon(FontIcon::symbol(FluentSymbol::Add)),
            MenuFlyoutItem::toggle("Autosave", app.get(self).autosave, move |app, value| {
                self.set_state(app, |state| state.autosave = value);
                self.refresh_menu(app);
            })
            .prevent_dismiss_on_pointer(true),
            MenuFlyoutItem::separator(),
            MenuFlyoutItem::radio(
                "List view",
                !app.get(self).grid_view,
                Listener::new(move |app| {
                    self.set_state(app, |state| state.grid_view = false);
                    self.refresh_menu(app);
                }),
            )
            .prevent_dismiss_on_pointer(true),
            MenuFlyoutItem::radio(
                "Grid view",
                app.get(self).grid_view,
                Listener::new(move |app| {
                    self.set_state(app, |state| state.grid_view = true);
                    self.refresh_menu(app);
                }),
            )
            .prevent_dismiss_on_pointer(true),
            MenuFlyoutItem::separator(),
            MenuFlyoutItem::sub_item("Export", vec![action("PDF document"), action("Plain text")]),
            action("Unavailable command").is_enabled(false),
        ]
    }
}

impl State for CommandDemoState {
    type Widget = CommandDemo;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let items = self.items(app);
        app.get_mut(self).menu = Some(MenuFlyout::new(app, items));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).menu.unwrap().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let flyout = app.get(self).menu.unwrap().as_flyout(app);
        let checked = app.get(self).checked;
        let click = Listener::new(move |app| self.set_state(app, |state| state.activations += 1));
        let feature = self.widget(app).feature;
        let controls = match feature {
            Feature::SplitButton => row(
                vec![
                    SplitButton::text("Save", click)
                        .flyout(flyout)
                        .into_widget(),
                    SplitButton::text("Disabled", Listener::new(|_| {}))
                        .flyout(flyout)
                        .is_enabled(false)
                        .into_widget(),
                ],
                16.0,
            ),
            Feature::ToggleSplitButton => row(
                vec![
                    ToggleSplitButton::text("Bold", checked, move |app, value| {
                        self.set_state(app, |state| state.checked = value);
                    })
                    .click(click)
                    .flyout(flyout)
                    .into_widget(),
                    ToggleSplitButton::text("Disabled", true, |_, _| {})
                        .flyout(flyout)
                        .is_enabled(false)
                        .into_widget(),
                ],
                16.0,
            ),
            _ => DropDownButton::text("Commands")
                .flyout(flyout)
                .into_widget(),
        };
        let description = if feature == Feature::MenuFlyout {
            "Use arrow keys to navigate. Export opens a submenu; Autosave and the view choices keep the menu open."
        } else {
            "The label performs the primary action. The arrow opens the menu. Tab focuses the control; Space activates it and F4 opens the menu."
        };
        column(
            vec![
                example("Actions and choices", description, &resources, controls),
                example(
                    "Current state",
                    "Menu commands and the primary action are independent.",
                    &resources,
                    column(
                        vec![
                            Text::new(format!(
                                "Primary actions: {} · checked: {}",
                                app.get(self).activations,
                                checked
                            ))
                            .into_widget(),
                            Text::new(format!(
                                "Autosave: {} · layout: {}",
                                app.get(self).autosave,
                                if app.get(self).grid_view {
                                    "Grid"
                                } else {
                                    "List"
                                }
                            ))
                            .into_widget(),
                            Text::new(format!("Last command: {}", app.get(self).last_action))
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

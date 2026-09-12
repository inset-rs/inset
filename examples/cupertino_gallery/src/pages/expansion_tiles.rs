//! `CupertinoExpansionTile`: a list tile that opens onto a body.
//!
//! Two things are worth watching. The transition mode decides what the body does while the
//! height animates, and the tile controller is a real handle: the button in the second section
//! drives the same tile the header tap does.

use inset_cupertino::{
    CupertinoButton, CupertinoColors, CupertinoDynamicColor, CupertinoExpansionTile,
    CupertinoIcons, CupertinoListTile, ExpansionTileTransitionMode,
};
use inset_foundation::{App, Handle, ListenableObject, Listener};
use inset_painting::AnyColor;
use inset_rendering::MainAxisSize;
use inset_widgets::{
    BuildContext, Column, ExpansibleController, IconData, IntoWidget, State, StateData,
    StatefulWidget, Text, WidgetRef,
};

use crate::support::{badge, screen, scrolling_body, secondary, section_with_footer, showcase};

/// The leading badge side, matching the one the Lists screen uses.
const BADGE_SIZE: f64 = 28.0;

/// The rows every tile on this screen opens onto.
fn rows_content() -> Vec<(&'static str, IconData, AnyColor)> {
    vec![
        (
            "Profile",
            CupertinoIcons::person(),
            CupertinoColors::SYSTEM_BLUE,
        ),
        (
            "Messages",
            CupertinoIcons::mail(),
            CupertinoColors::SYSTEM_GREEN,
        ),
        (
            "Settings",
            CupertinoIcons::settings(),
            CupertinoColors::SYSTEM_GREY,
        ),
    ]
}

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    ExpansionTilesDemo.into_widget()
}

#[derive(Debug)]
pub struct ExpansionTilesDemo;

/// Dart would spell this `_ExpansionTilesDemoState`.
pub struct DrivenTile {
    state: StateData<ExpansionTilesDemo>,
    /// Shared by the last section between its button and its tile, so the two drive one
    /// expansion rather than two.
    controller: Option<Handle<ExpansibleController>>,
}

impl StatefulWidget for ExpansionTilesDemo {
    type State = DrivenTile;

    fn create_state(&self) -> DrivenTile {
        DrivenTile {
            state: StateData::new(),
            controller: None,
        }
    }
}

impl DrivenTile {
    /// The controller is the expansion state; this page only mirrors it, so an empty
    /// `set_state` is the whole of Dart's `setState(() {})`.
    ///
    /// Without it the header tap would open the tile and leave the button's label and the
    /// readout below it reading the old value: the tile listens to the controller, and this
    /// page has to as well.
    fn handle_expansion_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }
}

impl State for DrivenTile {
    type Widget = ExpansionTilesDemo;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = ExpansibleController::new(app);
        controller.add_listener(
            app,
            Listener::handle_method(self, Self::handle_expansion_changed),
        );
        app.get_mut(self).controller = Some(controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).controller {
            controller.remove_listener(
                app,
                &Listener::handle_method(self, Self::handle_expansion_changed),
            );
            controller.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let controller = app
            .get(self)
            .controller
            .expect("init_state made the controller");
        let expanded = controller.is_expanded(app);
        let toggle = Listener::new(move |app: &mut App| controller.toggle(app));

        screen(scrolling_body(vec![
            section_with_footer(
                "Transition modes",
                "Scroll slides the body out from under the header; Fade cross-fades it while \
                 the height runs.",
                vec![
                    tile(
                        "Fade transition",
                        ExpansionTileTransitionMode::Fade,
                        None,
                        app,
                        context,
                    ),
                    tile(
                        "Scroll transition",
                        ExpansionTileTransitionMode::Scroll,
                        None,
                        app,
                        context,
                    ),
                ],
            ),
            section_with_footer(
                "Driven from outside",
                "The button and the header hold one ExpansibleController between them.",
                vec![
                    tile(
                        "Controlled",
                        ExpansionTileTransitionMode::Scroll,
                        Some(controller),
                        app,
                        context,
                    ),
                    showcase(CupertinoButton::filled(
                        Text::new(if expanded { "Collapse" } else { "Expand" }).into_widget(),
                        Some(toggle),
                    )),
                    showcase(secondary(app, context, format!("is_expanded: {expanded}"))),
                ],
            ),
        ]))
    }
}

fn tile(
    title: &str,
    transition_mode: ExpansionTileTransitionMode,
    controller: Option<Handle<ExpansibleController>>,
    app: &mut App,
    context: BuildContext,
) -> WidgetRef {
    let mut tile = CupertinoExpansionTile::new(Text::new(title), rows(app, context))
        .transition_mode(transition_mode);
    if let Some(controller) = controller {
        tile = tile.controller(controller);
    }
    tile.into_widget()
}

/// The body: a plain column, not a nested list section, so the rows read as a continuation of
/// the section the tile already sits in.
fn rows(app: &mut App, context: BuildContext) -> WidgetRef {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .children(
            rows_content()
                .into_iter()
                .map(|(label, icon, tint)| row(label, icon, &tint, app, context))
                .collect::<Vec<_>>(),
        )
        .into_widget()
}

fn row(
    label: &str,
    icon: IconData,
    tint: &AnyColor,
    app: &mut App,
    context: BuildContext,
) -> WidgetRef {
    let tint = CupertinoDynamicColor::resolve(tint, app, context);
    CupertinoListTile::notched(Text::new(label))
        .leading(badge(icon, tint, BADGE_SIZE))
        .into_widget()
}

//! ToolTip placement and shared hover/keyboard behavior.

use crate::{column, example, example_row, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Exercises automatic hover/focus opening and configurable placement.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    let automatic = example_row(
        [
            ("Hover or focus", PlacementMode::Top),
            ("Below", PlacementMode::Bottom),
            ("Beside", PlacementMode::Right),
        ]
        .into_iter()
        .map(|(label, placement)| {
            ToolTipService::new(
                Button::text(label, Listener::new(|_| {})),
                ToolTip::text("Additional information").placement(placement),
            )
            .into_widget()
        })
        .collect(),
        8.0,
    );
    section(
        "ToolTip",
        resources,
        column(
            vec![
                example(
                    "Automatic tooltips",
                    "Hover over or keyboard-focus a button. Each tooltip prefers a different side and falls back when it meets the window edge.",
                    resources,
                    automatic,
                ),
                example(
                    "Placement and explicit opening",
                    "Choose a preferred placement, then show the tooltip or hover over its target. Mouse placement follows the pointer for hover opening.",
                    resources,
                    PlacementDemo.into_widget(),
                ),
            ],
            20.0,
        ),
    )
}

/// Owns the configurable tooltip target.
#[derive(Debug)]
struct PlacementDemo;

/// Keeps placement and explicit opening independent from automatic hover state.
struct PlacementDemoState {
    /// Native widget lifecycle.
    state: StateData<PlacementDemo>,

    /// Preferred placement before edge fallback.
    placement: PlacementMode,

    /// Owner-requested tooltip visibility.
    open: bool,
}

impl StatefulWidget for PlacementDemo {
    type State = PlacementDemoState;

    fn create_state(&self) -> Self::State {
        PlacementDemoState {
            state: StateData::new(),
            placement: PlacementMode::Top,
            open: false,
        }
    }
}

impl State for PlacementDemoState {
    type Widget = PlacementDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let (placement, open) = (app.get(self).placement, app.get(self).open);
        let mut target = ToolTipService::new(
            Button::text("Placement target", Listener::new(|_| {})),
            ToolTip::text("A preferred side can change near the window edge.").placement(placement),
        );
        if open {
            target = target.is_open(true);
        }
        example_row(
            vec![
                Button::text(
                    format!("Tooltip placement: {placement:?}"),
                    Listener::new(move |app| {
                        self.set_state(app, |state| {
                            state.placement = match state.placement {
                                PlacementMode::Top => PlacementMode::Bottom,
                                PlacementMode::Bottom => PlacementMode::Left,
                                PlacementMode::Left => PlacementMode::Right,
                                PlacementMode::Right => PlacementMode::Mouse,
                                PlacementMode::Mouse => PlacementMode::Top,
                            };
                        })
                    }),
                )
                .into_widget(),
                CheckBox::new(Some(open), move |app, value| {
                    self.set_state(app, |state| state.open = value.unwrap_or(false))
                })
                .content(Text::new("Show tooltip explicitly"))
                .into_widget(),
                target.into_widget(),
            ],
            12.0,
        )
    }
}

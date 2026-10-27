//! Determinate and indeterminate ProgressRing examples.

use crate::{column, example, example_row, label, row, section};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::{CrossAxisAlignment, MainAxisSize};
use inset_widgets::*;
use inset_winui::*;

/// Builds the ring feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("ProgressRing", resources, RingDemo.into_widget())
}

/// Interactive ring properties.
#[derive(Debug)]
struct RingDemo;

/// Values shared by the determinate and indeterminate previews.
struct RingDemoState {
    /// Native widget linkage.
    state: StateData<RingDemo>,
    /// Completed percentage.
    value: f64,
    /// Whether the rings are shown at all.
    active: bool,
}

impl StatefulWidget for RingDemo {
    type State = RingDemoState;

    fn create_state(&self) -> Self::State {
        RingDemoState {
            state: StateData::new(),
            value: 40.0,
            active: true,
        }
    }
}

/// A ring above its caption.
fn captioned(ring: WidgetRef, name: &str, resources: &ThemeResources) -> WidgetRef {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .spacing(10.0)
        .children([
            ring,
            label(
                name,
                TextBlockStyle::Caption,
                resources.text_fill_color_secondary,
            ),
        ])
        .into_widget()
}

impl State for RingDemoState {
    type Widget = RingDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let (value, active) = {
            let state = app.get(self);
            (state.value, state.active)
        };
        let options = row(
            vec![
                Button::text(
                    format!("Value: {value:.0}%"),
                    Listener::new(move |app| {
                        self.set_state(app, |s| s.value = (s.value + 20.0) % 120.0)
                    }),
                )
                .into_widget(),
                CheckBox::new(Some(active), move |app, checked| {
                    self.set_state(app, |s| s.active = checked.unwrap_or(false))
                })
                .content(Text::new("Active"))
                .into_widget(),
            ],
            16.0,
        );
        let rings = row(
            vec![
                captioned(
                    ProgressRing::new()
                        .is_indeterminate(false)
                        .value(value)
                        .is_active(active)
                        .into_widget(),
                    "Determinate",
                    &r,
                ),
                captioned(
                    ProgressRing::new().is_active(active).into_widget(),
                    "Indeterminate",
                    &r,
                ),
            ],
            40.0,
        );
        column(
            vec![
                example(
                    "Progress",
                    "A determinate ring fills to its value; an indeterminate one turns for as \
                     long as the work lasts. Clearing Active hides both.",
                    &r,
                    example_row(vec![options, rings], 24.0),
                ),
                example(
                    "Colors",
                    "Foreground draws the arc and Background the track behind it, which the \
                     Fluent style leaves transparent.",
                    &r,
                    row(
                        vec![
                            ProgressRing::new()
                                .is_indeterminate(false)
                                .value(75.0)
                                .background(r.control_strong_fill_color_default)
                                .into_widget(),
                            ProgressRing::new()
                                .is_indeterminate(false)
                                .value(75.0)
                                .foreground(r.system_fill_color_success_brush)
                                .background(r.control_strong_fill_color_default)
                                .into_widget(),
                        ],
                        24.0,
                    ),
                ),
            ],
            24.0,
        )
    }
}

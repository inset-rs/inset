//! `Slider`: horizontal with a header and a value label, stepped with tick marks, disabled, and vertical.
use crate::{column, example, example_row, label, row, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
// `Orientation` is also a `inset_widgets` name (the media query's); the slider's is XAML's.
use inset_winui::Orientation;
use inset_winui::*;

/// The track height for the vertical orientation example.
const VERTICAL_DEMO_HEIGHT: f64 = 184.0;

/// Builds the examples for this feature destination.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("Slider", resources, SliderDemo.into_widget())
}

/// Owns the independent interactive examples on this page.
#[derive(Debug)]
struct SliderDemo;

/// Values and property choices retained while this page is visited.
struct SliderDemoState {
    /// Native state binding for the slider examples.
    state: StateData<SliderDemo>,

    /// Value of the continuous horizontal example.
    volume: f64,

    /// Value of the stepped horizontal example.
    stepped: f64,

    /// Value of the vertical example.
    vertical: f64,

    /// Value in the configurable range example.
    sample: f64,

    /// Reverses the configurable slider's direction.
    reversed: bool,

    /// Selects tick-based snapping rather than step-based snapping.
    snap_ticks: bool,

    /// Shows tick marks on the configurable slider.
    ticks: bool,

    /// Enables its converted thumb tooltip.
    tooltip: bool,

    /// Prevents input without resetting the example value.
    enabled: bool,
}

impl StatefulWidget for SliderDemo {
    type State = SliderDemoState;

    /// Initializes each example with a distinct useful starting value.
    fn create_state(&self) -> SliderDemoState {
        SliderDemoState {
            state: StateData::new(),
            volume: 42.0,
            stepped: 50.0,
            vertical: 30.0,
            sample: 20.0,
            reversed: false,
            snap_ticks: false,
            ticks: true,
            tooltip: true,
            enabled: true,
        }
    }
}

impl State for SliderDemoState {
    type Widget = SliderDemo;
    inset_widgets::state_accessors!();

    /// Builds the examples from their current property choices.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let text = resources.text_fill_color_primary;
        let (volume, stepped, vertical) = {
            let state = app.get(self);
            (state.volume, state.stepped, state.vertical)
        };
        let value_label = |value: f64| label(format!("{value:.0}"), TextBlockStyle::Body, text);
        let horizontal = column(
            vec![
                column(
                    vec![
                        SizedBox::new()
                            .width(f64::INFINITY)
                            .child(
                                Slider::new(volume, move |app, value| {
                                    self.set_state(app, |s| s.volume = value)
                                })
                                .header(Text::new("Volume")),
                            )
                            .into_widget(),
                        value_label(volume),
                    ],
                    12.0,
                ),
                column(
                    vec![
                        SizedBox::new()
                            .width(f64::INFINITY)
                            .child(
                                Slider::new(stepped, move |app, value| {
                                    self.set_state(app, |s| s.stepped = value)
                                })
                                .header(Text::new("Stepped"))
                                .step_frequency(10.0)
                                .tick_frequency(10.0)
                                .tick_placement(TickPlacement::Outside),
                            )
                            .into_widget(),
                        value_label(stepped),
                    ],
                    12.0,
                ),
                SizedBox::new()
                    .width(f64::INFINITY)
                    .child(
                        Slider::new(70.0, |_, _| {})
                            .header(Text::new("Disabled"))
                            .is_enabled(false),
                    )
                    .into_widget(),
            ],
            8.0,
        );
        let vertical_demo = row(
            vec![
                SizedBox::new()
                    .width(64.0)
                    .height(VERTICAL_DEMO_HEIGHT)
                    .child(
                        Slider::new(vertical, move |app, value| {
                            self.set_state(app, |s| s.vertical = value)
                        })
                        .header(Text::new("Vertical"))
                        .orientation(Orientation::Vertical),
                    )
                    .into_widget(),
                value_label(vertical),
            ],
            12.0,
        );
        let (sample, reversed, snap_ticks, ticks, tooltip, enabled) = {
            let state = app.get(self);
            (
                state.sample,
                state.reversed,
                state.snap_ticks,
                state.ticks,
                state.tooltip,
                state.enabled,
            )
        };
        column(
            vec![
                example(
                    "Horizontal values",
                    "Drag a thumb or focus it and use the arrow keys. The stepped example moves in increments of ten.",
                    &resources,
                    horizontal,
                ),
                example(
                    "Vertical orientation",
                    "The same value control arranged vertically.",
                    &resources,
                    vertical_demo,
                ),
                label(
                    format!("Slider: {volume:.0} · stepped {stepped:.0} · vertical {vertical:.0}"),
                    TextBlockStyle::Body,
                    resources.text_fill_color_secondary,
                ),
                example(
                    "Range, snapping and tooltips",
                    "This range runs from −50 to 50. Step values are 5 apart; tick values are 10 apart. Hover or drag the thumb to see its converted value.",
                    &resources,
                    column(
                        vec![
                            example_row(
                                vec![
                                    CheckBox::new(Some(reversed), move |app, value| {
                                        self.set_state(app, |s| s.reversed = value == Some(true))
                                    })
                                    .content(Text::new("Reverse direction"))
                                    .into_widget(),
                                    CheckBox::new(Some(snap_ticks), move |app, value| {
                                        self.set_state(app, |s| s.snap_ticks = value == Some(true))
                                    })
                                    .content(Text::new("Snap to ticks"))
                                    .into_widget(),
                                    CheckBox::new(Some(ticks), move |app, value| {
                                        self.set_state(app, |s| s.ticks = value == Some(true))
                                    })
                                    .content(Text::new("Show tick marks"))
                                    .into_widget(),
                                ],
                                16.0,
                            ),
                            example_row(
                                vec![
                                    CheckBox::new(Some(tooltip), move |app, value| {
                                        self.set_state(app, |s| s.tooltip = value == Some(true))
                                    })
                                    .content(Text::new("Show thumb tooltip"))
                                    .into_widget(),
                                    CheckBox::new(Some(enabled), move |app, value| {
                                        self.set_state(app, |s| s.enabled = value == Some(true))
                                    })
                                    .content(Text::new("Enable range slider"))
                                    .into_widget(),
                                ],
                                16.0,
                            ),
                            Slider::new(sample, move |app, value| {
                                self.set_state(app, |s| s.sample = value)
                            })
                            .minimum(-50.0)
                            .maximum(50.0)
                            .step_frequency(5.0)
                            .small_change(5.0)
                            .tick_frequency(10.0)
                            .tick_placement(if ticks {
                                TickPlacement::Outside
                            } else {
                                TickPlacement::None
                            })
                            .snaps_to(if snap_ticks {
                                SliderSnapsTo::Ticks
                            } else {
                                SliderSnapsTo::StepValues
                            })
                            .is_direction_reversed(reversed)
                            .is_enabled(enabled)
                            .is_thumb_tool_tip_enabled(tooltip)
                            .thumb_tool_tip_value_converter(|value| format!("{value:+.0} dB"))
                            .header(Text::new("Gain"))
                            .into_widget(),
                            example_row(
                                vec![
                                    Button::text(
                                        "Set gain to minimum",
                                        Listener::new(move |app| {
                                            self.set_state(app, |s| s.sample = -50.0)
                                        }),
                                    )
                                    .into_widget(),
                                    Button::text(
                                        "Reset gain",
                                        Listener::new(move |app| {
                                            self.set_state(app, |s| s.sample = 0.0)
                                        }),
                                    )
                                    .into_widget(),
                                    Button::text(
                                        "Set gain to maximum",
                                        Listener::new(move |app| {
                                            self.set_state(app, |s| s.sample = 50.0)
                                        }),
                                    )
                                    .into_widget(),
                                ],
                                12.0,
                            ),
                            label(
                                format!("Gain: {sample:+.0} dB"),
                                TextBlockStyle::Body,
                                resources.text_fill_color_secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
            ],
            16.0,
        )
    }
}

//! `CupertinoSegmentedControl`: one choice out of a small mutually exclusive set.
//!
//! The control stores no selection. `on_value_changed` reports the tapped key and nothing moves
//! until this page rebuilds it with a new `group_value` — which is why the preview card, reading
//! the same field as the control, follows a tap on either control.

use std::rc::Rc;

use indexmap::IndexMap;
use inset_cupertino::{CupertinoColors, CupertinoSegmentedControl};
use inset_embedder::Color;
use inset_foundation::{App, Handle, ValueChanged};
use inset_painting::{BorderRadiusGeometry, BoxDecoration, EdgeInsetsGeometry};
use inset_widgets::{
    BuildContext, Container, DefaultTextStyle, IntoWidget, Padding, State, StateData,
    StatefulWidget, Text, WidgetRef,
};

use crate::support::{body_style, screen, scrolling_body, section_with_footer, showcase};

/// The three options every control on this page offers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Sky {
    Midnight,
    Viridian,
    Cerulean,
}

impl Sky {
    const ALL: [Sky; 3] = [Sky::Midnight, Sky::Viridian, Sky::Cerulean];

    fn label(self) -> &'static str {
        match self {
            Sky::Midnight => "Midnight",
            Sky::Viridian => "Viridian",
            Sky::Cerulean => "Cerulean",
        }
    }

    /// What fills the preview card, so the selection shows somewhere other than the highlight
    /// inside the control.
    fn color(self) -> Color {
        match self {
            Sky::Midnight => Color::from_argb(0xFF, 0x19, 0x19, 0x70),
            Sky::Viridian => Color::from_argb(0xFF, 0x40, 0x82, 0x6d),
            Sky::Cerulean => Color::from_argb(0xFF, 0x00, 0x7b, 0xa7),
        }
    }
}

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    SegmentsDemo.into_widget()
}

/// Both selections live here, in the smallest widget that renders them.
#[derive(Debug)]
pub struct SegmentsDemo;

/// Dart would spell this `_SegmentsDemoState`.
pub struct Selections {
    state: StateData<SegmentsDemo>,
    /// The first two controls read this one, so a tap on either moves both.
    sky: Sky,
    /// The third control keeps its own, so it can rest on a value the shared pair is not on.
    limited: Sky,
}

impl StatefulWidget for SegmentsDemo {
    type State = Selections;

    fn create_state(&self) -> Selections {
        Selections {
            state: StateData::new(),
            sky: Sky::Midnight,
            limited: Sky::Viridian,
        }
    }
}

impl State for Selections {
    type Widget = SegmentsDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let choose: ValueChanged<Sky> = Rc::new(move |app: &mut App, sky| {
            self.set_state(app, |state| state.sky = sky);
        });
        let choose_limited: ValueChanged<Sky> = Rc::new(move |app: &mut App, sky| {
            self.set_state(app, |state| state.limited = sky);
        });
        let (sky, limited) = {
            let state = app.get(self);
            (state.sky, state.limited)
        };

        screen(scrolling_body(vec![
            section_with_footer(
                "Selection",
                "on_value_changed reports the tap; group_value is what draws the highlight.",
                vec![
                    showcase(styled(control(sky, choose.clone()), app, context)),
                    preview(sky, app, context),
                ],
            ),
            section_with_footer(
                "The same value, a second control",
                "Neither control stores it: both read one field of this page, so both move.",
                vec![showcase(styled(tinted(sky, choose), app, context))],
            ),
            section_with_footer(
                "Disabled segments",
                "A key in disabled_children takes no tap and draws in the disabled colours.",
                vec![showcase(styled(
                    control(limited, choose_limited).disabled_children([Sky::Midnight]),
                    app,
                    context,
                ))],
            ),
        ]))
    }
}

/// The control under the body text style of the theme.
///
/// A segment label reads the ambient `DefaultTextStyle` and the control overrides only its
/// colour, so a segment inside a list section — which installs the section's own header style
/// around its children — reads as the section, not as a button.
fn styled(
    control: CupertinoSegmentedControl<Sky>,
    app: &mut App,
    context: BuildContext,
) -> WidgetRef {
    DefaultTextStyle::new(body_style(app, context), control).into_widget()
}

fn control(selected: Sky, on_value_changed: ValueChanged<Sky>) -> CupertinoSegmentedControl<Sky> {
    CupertinoSegmentedControl::new(segments(), on_value_changed)
        .group_value(selected)
        // The default is 16 points either side, on top of the section inset and the padding
        // `showcase` adds. One of the three is enough.
        .padding(EdgeInsetsGeometry::ZERO)
}

/// The same control in the colours of the selected sky.
fn tinted(selected: Sky, on_value_changed: ValueChanged<Sky>) -> CupertinoSegmentedControl<Sky> {
    control(selected, on_value_changed)
        .selected_color(selected.color().into())
        .border_color(selected.color().into())
        .unselected_color(CupertinoColors::WHITE)
}

fn segments() -> IndexMap<Sky, WidgetRef> {
    Sky::ALL
        .iter()
        .map(|sky| {
            (
                *sky,
                Padding::new(EdgeInsetsGeometry::symmetric(6.0, 12.0))
                    .child(Text::new(sky.label()))
                    .into_widget(),
            )
        })
        .collect()
}

fn preview(sky: Sky, app: &mut App, context: BuildContext) -> WidgetRef {
    showcase(
        Container::new()
            .padding(EdgeInsetsGeometry::symmetric(16.0, 24.0))
            .decoration(
                BoxDecoration::new()
                    .color(sky.color())
                    .border_radius(BorderRadiusGeometry::circular(10.0)),
            )
            .child(
                Text::new(format!("Selected: {}", sky.label()))
                    .style(body_style(app, context).color(CupertinoColors::WHITE)),
            ),
    )
}

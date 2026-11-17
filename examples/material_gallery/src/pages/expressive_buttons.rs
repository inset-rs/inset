//! The first Material 3 Expressive components the port carries: split buttons in their four
//! styles and two states, and button groups, standard and connected.

use waft_foundation::{App, Handle, Listener};
use waft_material::{
    ButtonGroup, ButtonGroupDefaults, ButtonStyle, FilledButton, Icons, SplitButtonDefaults,
    SplitButtonLayout, SplitButtonLeadingButton, SplitButtonTrailingButton, SplitButtonVariant,
};
use waft_painting::OutlinedBorderRef;
use waft_widgets::{
    BuildContext, Icon, IntoWidget, ListView, Row, SizedBox, State, StateData, StatefulWidget,
    Text, WidgetRef, WidgetState, WidgetStateProperty,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::ExpressiveButtons.title(), ExpressiveButtonsDemo).into_widget()
}

/// The split buttons' checked state and the number of toggles.
#[derive(Debug)]
pub struct ExpressiveButtonsDemo;

pub struct ExpressiveButtonsDemoState {
    state: StateData<ExpressiveButtonsDemo>,
    checked: bool,
    toggles: usize,
}

impl StatefulWidget for ExpressiveButtonsDemo {
    type State = ExpressiveButtonsDemoState;

    fn create_state(&self) -> ExpressiveButtonsDemoState {
        ExpressiveButtonsDemoState {
            state: StateData::new(),
            checked: false,
            toggles: 0,
        }
    }
}

fn leading(on_pressed: Option<Listener>) -> SplitButtonLeadingButton {
    SplitButtonLeadingButton::new(
        on_pressed,
        Row::new()
            .children(vec![
                Icon::new(Some(Icons::edit()))
                    .size(SplitButtonDefaults::leading_button_icon_size_for(
                        SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
                    ))
                    .into_widget(),
                SizedBox::new().width(8.0).into_widget(),
                Text::new("Compose").into_widget(),
            ])
            .into_widget(),
    )
}

fn trailing(checked: bool, on_pressed: Option<Listener>) -> SplitButtonTrailingButton {
    SplitButtonTrailingButton::toggle(
        checked,
        on_pressed,
        Icon::new(Some(Icons::arrow_drop_down())).size(
            SplitButtonDefaults::trailing_button_icon_size_for(
                SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
            ),
        ),
    )
}

/// A group button styled with a connected group's three shapes.
fn connected_button(
    label: &'static str,
    (resting, pressed, checked): (OutlinedBorderRef, OutlinedBorderRef, OutlinedBorderRef),
) -> WidgetRef {
    let shape = <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_with(
        move |states: &waft_widgets::WidgetStates| {
            if states.contains(&WidgetState::Pressed) {
                Some(pressed.clone())
            } else if states.contains(&WidgetState::Selected) {
                Some(checked.clone())
            } else {
                Some(resting.clone())
            }
        },
    );
    FilledButton::new(
        Some(Listener::new(|_app: &mut App| {})),
        Some(Text::new(label)),
    )
    .style(ButtonStyle::new().shape(shape))
    .into_widget()
}

impl State for ExpressiveButtonsDemoState {
    type Widget = ExpressiveButtonsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let checked = app.get(self).checked;
        let toggles = app.get(self).toggles;
        let on_toggle: Listener = Listener::new(move |app: &mut App| {
            let checked = app.get(self).checked;
            app.get_mut(self).checked = !checked;
            app.get_mut(self).toggles += 1;
            self.set_state(app, |_state| {});
        });

        let mut split_buttons: Vec<WidgetRef> = Vec::new();
        for variant in [
            SplitButtonVariant::Filled,
            SplitButtonVariant::Tonal,
            SplitButtonVariant::Elevated,
            SplitButtonVariant::Outlined,
        ] {
            split_buttons.push(
                SplitButtonLayout::new(
                    leading(Some(Listener::new(|_app| {}))).variant(variant),
                    trailing(checked, Some(on_toggle.clone())).variant(variant),
                )
                .into_widget(),
            );
        }

        let leading_shapes = ButtonGroupDefaults::connected_leading_button_shapes_for(
            SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
        );
        let middle_shapes = ButtonGroupDefaults::connected_middle_button_shapes_for(
            SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
        );
        let trailing_shapes = ButtonGroupDefaults::connected_trailing_button_shapes_for(
            SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
        );

        ListView::new()
            .children(vec![
                section_header(app, context, "Split buttons"),
                showcase(split_buttons),
                caption(
                    app,
                    context,
                    match toggles {
                        0 => "Filled, tonal, elevated and outlined; the trailing part toggles."
                            .to_string(),
                        toggles => format!("Trailing toggled {toggles} times"),
                    },
                ),
                section_header(app, context, "A disabled split button"),
                showcase(vec![
                    SplitButtonLayout::new(leading(None), trailing(false, None)).into_widget(),
                ]),
                section_header(app, context, "Button groups"),
                showcase(vec![
                    ButtonGroup::new(vec![
                        FilledButton::new(Some(Listener::new(|_app| {})), Some(Text::new("One")))
                            .into_widget(),
                        FilledButton::new(Some(Listener::new(|_app| {})), Some(Text::new("Two")))
                            .into_widget(),
                        FilledButton::new(Some(Listener::new(|_app| {})), Some(Text::new("Three")))
                            .into_widget(),
                    ])
                    .into_widget(),
                ]),
                caption(app, context, "Standard, 12 apart."),
                showcase(vec![
                    ButtonGroup::connected(vec![
                        connected_button("Left", leading_shapes),
                        connected_button("Middle", middle_shapes),
                        connected_button("Right", trailing_shapes),
                    ])
                    .into_widget(),
                ]),
                caption(
                    app,
                    context,
                    "Connected, 2 apart, with the inner corners tighter.",
                ),
            ])
            .into_widget()
    }
}

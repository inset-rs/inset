//! Retained content, placement and outside-input examples for Flyout and DropDownButton.

use crate::{column, example, row, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;
use std::rc::Rc;

/// Builds the Flyout feature page.
pub fn flyout(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "Flyout",
        resources,
        FlyoutDemo { dropdown: false }.into_widget(),
    )
}

/// Builds the DropDownButton feature page.
pub fn drop_down_button(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "DropDownButton",
        resources,
        FlyoutDemo { dropdown: true }.into_widget(),
    )
}

/// Separate pages share examples while owning independent flyout instances.
#[derive(Debug)]
struct FlyoutDemo {
    /// Whether opening uses the dropdown button template.
    dropdown: bool,
}

/// Owns flyout controllers and the settings shown beside the opening buttons.
struct FlyoutDemoState {
    /// Native widget state.
    state: StateData<FlyoutDemo>,

    /// Shared by two opening targets to demonstrate content retention.
    flyout: Option<Handle<Flyout>>,

    /// Current preferred placement.
    placement: FlyoutPlacementMode,

    /// Current focus and outside-input policy.
    mode: FlyoutShowMode,

    /// Prevents dismissal when requested by the example checkbox.
    prevent_close: bool,

    /// Number of activations of the outside example button.
    outside_clicks: usize,
}

impl StatefulWidget for FlyoutDemo {
    type State = FlyoutDemoState;

    fn create_state(&self) -> Self::State {
        FlyoutDemoState {
            state: StateData::new(),
            flyout: None,
            placement: FlyoutPlacementMode::Top,
            mode: FlyoutShowMode::Standard,
            prevent_close: false,
            outside_clicks: 0,
        }
    }
}

impl State for FlyoutDemoState {
    type Widget = FlyoutDemo;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let flyout = Flyout::new(app, RetainedCounter);
        app.get_mut(flyout).closing = Some(Rc::new(move |app, args| {
            args.cancel = app.get(self).prevent_close;
        }));
        app.get_mut(self).flyout = Some(flyout);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).flyout.unwrap().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let widget = self.widget(app);
        let dropdown = widget.dropdown;
        let state = app.get(self);
        let flyout = state.flyout.unwrap();
        let placement = state.placement;
        let mode = state.mode;
        let prevent_close = state.prevent_close;
        let outside_clicks = state.outside_clicks;
        let opener = move |text: &'static str| -> WidgetRef {
            if dropdown {
                DropDownButton::text(text).flyout(flyout).into_widget()
            } else {
                FlyoutTarget::new(Builder::new(move |_, context| {
                    Button::text(text, Listener::new(move |app| flyout.show_at(app, context)))
                        .into_widget()
                }))
                .into_widget()
            }
        };
        let mut examples = vec![example(
            "Content and placement",
            "The counter keeps its value when closed and when opened from the other button. Placement can fall back near a window edge.",
            &resources,
            column(
                vec![
                    Button::text(
                        format!("Placement: {placement:?}"),
                        Listener::new(move |app| {
                            let next = match app.get(self).placement {
                                FlyoutPlacementMode::Top => FlyoutPlacementMode::Bottom,
                                FlyoutPlacementMode::Bottom => FlyoutPlacementMode::Left,
                                FlyoutPlacementMode::Left => FlyoutPlacementMode::Right,
                                FlyoutPlacementMode::Right => FlyoutPlacementMode::Full,
                                _ => FlyoutPlacementMode::Top,
                            };
                            flyout.placement(app, next);
                            self.set_state(app, |state| state.placement = next);
                        }),
                    )
                    .into_widget(),
                    row(vec![opener("Open here"), opener("Open from here")], 24.0),
                ],
                16.0,
            ),
        )];
        examples.push(example(
            "Dismissal and outside input",
            "Standard mode blocks the click that dismisses it. Transient mode lets that click reach the control behind it.",
            &resources,
            column(vec![
                Button::text(format!("Mode: {mode:?}"), Listener::new(move |app| {
                    let next = match app.get(self).mode {
                        FlyoutShowMode::Standard => FlyoutShowMode::Transient,
                        FlyoutShowMode::Transient => FlyoutShowMode::TransientWithDismissOnPointerMoveAway,
                        _ => FlyoutShowMode::Standard,
                    };
                    flyout.show_mode(app, next);
                    self.set_state(app, |state| state.mode = next);
                })).into_widget(),
                CheckBox::new(Some(prevent_close), move |app, checked| {
                    self.set_state(app, |state| state.prevent_close = checked == Some(true));
                }).content(Text::new("Cancel Closing")).into_widget(),
                Button::text(format!("Outside action: {outside_clicks}"), Listener::new(move |app| {
                    self.set_state(app, |state| state.outside_clicks += 1);
                })).into_widget(),
            ], 16.0),
        ));
        if dropdown {
            examples.push(example(
                "Disabled",
                "A disabled button cannot open its flyout.",
                &resources,
                DropDownButton::text("Unavailable")
                    .flyout(flyout)
                    .is_enabled(false)
                    .into_widget(),
            ));
        }
        column(examples, 24.0)
    }
}

/// State inside the flyout belongs to its retained content, not either opening button.
#[derive(Debug)]
struct RetainedCounter;

/// Counter value survives closing the flyout.
struct RetainedCounterState {
    /// Native widget state.
    state: StateData<RetainedCounter>,

    /// Number of activations inside the flyout.
    count: usize,
}

impl StatefulWidget for RetainedCounter {
    type State = RetainedCounterState;

    fn create_state(&self) -> Self::State {
        RetainedCounterState {
            state: StateData::new(),
            count: 0,
        }
    }
}

impl State for RetainedCounterState {
    type Widget = RetainedCounter;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        column(
            vec![
                Text::new("This content is shared by both opening buttons.").into_widget(),
                Button::text(
                    format!("Count: {}", app.get(self).count),
                    Listener::new(move |app| {
                        self.set_state(app, |state| state.count += 1);
                    }),
                )
                .into_widget(),
            ],
            12.0,
        )
    }
}

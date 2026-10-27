//! `ToggleSwitch` with a header, with on and off content, and disabled.
use crate::{GalleryState, column, example, example_row, label, section};
use inset_foundation::{App, Handle};
use inset_widgets::*;
use inset_winui::*;

/// Builds shared gallery switches alongside an independent property example.
pub fn build(state: Handle<GalleryState>, app: &App, resources: &ThemeResources) -> Vec<WidgetRef> {
    let (wifi, airplane) = (app.get(state).wifi, app.get(state).airplane);
    section(
        "ToggleSwitch",
        resources,
        column(
            vec![
                example(
                    "Headers and state content",
                    "These switches share the gallery's Wi-Fi and airplane-mode state.",
                    resources,
                    column(
                        vec![
                            ToggleSwitch::new(wifi, move |app, on| {
                                state.set_state(app, |s| s.wifi = on)
                            })
                            .header(Text::new("Wi-Fi"))
                            .into_widget(),
                            ToggleSwitch::new(airplane, move |app, on| {
                                state.set_state(app, |s| s.airplane = on)
                            })
                            .on_content(Text::new("Airplane mode on"))
                            .off_content(Text::new("Airplane mode off"))
                            .into_widget(),
                            ToggleSwitch::new(true, |_, _| {})
                                .is_enabled(false)
                                .header(Text::new("Locked"))
                                .into_widget(),
                        ],
                        12.0,
                    ),
                ),
                ToggleProperties.into_widget(),
            ],
            16.0,
        ),
    )
}

/// An independent switch with configurable header, content and enabled state.
#[derive(Debug)]
struct ToggleProperties;

/// Property choices and value of the independent switch.
struct TogglePropertiesState {
    /// Native state binding for this example.
    state: StateData<ToggleProperties>,

    /// Current notification preference.
    on: bool,

    /// Whether notification preference can be changed.
    enabled: bool,

    /// Whether the switch displays a header.
    header: bool,

    /// Whether the default On/Off text is replaced.
    custom_content: bool,
}

impl StatefulWidget for ToggleProperties {
    type State = TogglePropertiesState;

    /// Creates property choices independently from the shared gallery values.
    fn create_state(&self) -> Self::State {
        TogglePropertiesState {
            state: StateData::new(),
            on: false,
            enabled: true,
            header: true,
            custom_content: true,
        }
    }
}

impl State for TogglePropertiesState {
    type Widget = ToggleProperties;
    inset_widgets::state_accessors!();

    /// Rebuilds the switch from the selected properties.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let s = app.get(self);
        let (on, enabled, header, custom_content) = (s.on, s.enabled, s.header, s.custom_content);
        let mut switch =
            ToggleSwitch::new(on, move |app, value| self.set_state(app, |s| s.on = value))
                .is_enabled(enabled);
        if header {
            switch = switch.header(Text::new("Notifications"));
        }
        if custom_content {
            switch = switch
                .on_content(Text::new("Notifications enabled"))
                .off_content(Text::new("Notifications paused"));
        }
        example(
            "Configure a switch",
            "Toggle the properties to compare the default and custom content.",
            &resources,
            column(
                vec![
                    example_row(
                        vec![
                            CheckBox::new(Some(enabled), move |app, value| {
                                self.set_state(app, |s| s.enabled = value == Some(true))
                            })
                            .content(Text::new("Enable notification switch"))
                            .into_widget(),
                            CheckBox::new(Some(header), move |app, value| {
                                self.set_state(app, |s| s.header = value == Some(true))
                            })
                            .content(Text::new("Show notification header"))
                            .into_widget(),
                            CheckBox::new(Some(custom_content), move |app, value| {
                                self.set_state(app, |s| s.custom_content = value == Some(true))
                            })
                            .content(Text::new("Use custom On/Off content"))
                            .into_widget(),
                        ],
                        16.0,
                    ),
                    switch.into_widget(),
                    label(
                        format!("Notification preference: {}", if on { "On" } else { "Off" }),
                        TextBlockStyle::Body,
                        resources.text_fill_color_secondary,
                    ),
                ],
                12.0,
            ),
        )
    }
}

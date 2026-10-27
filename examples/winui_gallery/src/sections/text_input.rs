//! Independent text and password examples with retained native controllers.

use crate::{column, example, example_row, label, section};
use inset_embedder::TextSelection;
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the TextBox feature page.
pub fn text_box(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "TextBox",
        resources,
        TextExamples { password: false }.into_widget(),
    )
}

/// Builds the PasswordBox feature page.
pub fn password_box(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "PasswordBox",
        resources,
        TextExamples { password: true }.into_widget(),
    )
}

/// Selects the examples while their controllers remain local to the page.
#[derive(Debug)]
struct TextExamples {
    /// Chooses PasswordBox examples instead of TextBox examples.
    password: bool,
}

/// Controllers and property choices survive navigation away from this page.
struct TextExamplesState {
    /// Native widget state identity.
    state: StateData<TextExamples>,

    /// Independent controllers for the examples.
    controllers: Vec<Handle<TextEditingController>>,

    /// Interactive enabled-state option.
    enabled: bool,

    /// Interactive read-only option for the TextBox.
    read_only: bool,

    /// Source password reveal mode.
    reveal_mode: PasswordRevealMode,

    /// Number of user edits, without storing password contents in status text.
    edits: usize,
}

impl StatefulWidget for TextExamples {
    type State = TextExamplesState;

    fn create_state(&self) -> Self::State {
        TextExamplesState {
            state: StateData::new(),
            controllers: Vec::new(),
            enabled: true,
            read_only: false,
            reveal_mode: PasswordRevealMode::Peek,
            edits: 0,
        }
    }
}

impl State for TextExamplesState {
    type Widget = TextExamples;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let initial = if self.widget(app).password {
            ["", "Saved password", ""]
        } else {
            ["", "This text is read-only.", ""]
        };
        let controllers = initial
            .into_iter()
            .map(|text| TextEditingController::new(app).text(app, text))
            .collect();
        app.get_mut(self).controllers = controllers;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for controller in app.get(self).controllers.clone() {
            app.get_mut(controller).dispose();
            app.destroy(controller);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let (controllers, enabled, read_only, mode, edits) = {
            let s = app.get(self);
            (
                s.controllers.clone(),
                s.enabled,
                s.read_only,
                s.reveal_mode,
                s.edits,
            )
        };
        let changed = move |app: &mut App, _: String| self.set_state(app, |s| s.edits += 1);
        let enable = CheckBox::new(Some(enabled), move |app, value| {
            self.set_state(app, |s| s.enabled = value == Some(true))
        })
        .content(Text::new("Enabled"));
        if self.widget(app).password {
            let mode_label = format!("Reveal mode: {mode:?}");
            let change_mode = Button::text(
                mode_label,
                Listener::new(move |app| {
                    self.set_state(app, |s| {
                        s.reveal_mode = match s.reveal_mode {
                            PasswordRevealMode::Peek => PasswordRevealMode::Hidden,
                            PasswordRevealMode::Hidden => PasswordRevealMode::Visible,
                            PasswordRevealMode::Visible => PasswordRevealMode::Peek,
                        }
                    })
                }),
            );

            column(
                vec![
                    example(
                        "Password reveal",
                        "Enter a new password to enable the eye, then hold it to reveal. Alt+F8 also peeks.",
                        &r,
                        column(
                            vec![
                                example_row(
                                    vec![enable.into_widget(), change_mode.into_widget()],
                                    16.0,
                                ),
                                PasswordBox::new(controllers[0].as_controller())
                                    .header(Text::new("Password"))
                                    .placeholder_text("Enter a password")
                                    .description(Text::new(
                                        "The eye is hidden when you return to an existing password.",
                                    ))
                                    .is_enabled(enabled)
                                    .password_reveal_mode(mode)
                                    .password_changed(changed)
                                    .into_widget(),
                                label(
                                    format!("User edits: {edits}"),
                                    TextBlockStyle::Caption,
                                    r.text_fill_color_secondary,
                                ),
                            ],
                            16.0,
                        ),
                    ),
                    example(
                        "Disabled and limited input",
                        "A disabled editor retains its content. The second editor accepts at most 12 characters.",
                        &r,
                        column(
                            vec![
                                PasswordBox::new(controllers[1].as_controller())
                                    .header(Text::new("Stored password"))
                                    .is_enabled(false)
                                    .into_widget(),
                                PasswordBox::new(controllers[2].as_controller())
                                    .header(Text::new("Limited password"))
                                    .placeholder_text("Up to 12 characters")
                                    .max_length(12)
                                    .into_widget(),
                            ],
                            20.0,
                        ),
                    ),
                ],
                20.0,
            )
        } else {
            let controller = controllers[0];

            column(
                vec![
                    example(
                        "Text and selection",
                        "Click to edit, drag or double-click to select, and use the context menu or keyboard shortcuts.",
                        &r,
                        column(
                            vec![
                                example_row(
                                    vec![
                                        enable.into_widget(),
                                        CheckBox::new(Some(read_only), move |app, value| {
                                            self.set_state(app, |s| s.read_only = value == Some(true))
                                        })
                                        .content(Text::new("Read-only"))
                                        .into_widget(),
                                        Button::text(
                                            "Select all text",
                                            Listener::new(move |app| {
                                                let length = controller
                                                    .text_value(app)
                                                    .encode_utf16()
                                                    .count() as i32;

                                                controller.set_selection(
                                                    app,
                                                    TextSelection::new(0, length),
                                                );
                                            }),
                                        )
                                        .into_widget(),
                                    ],
                                    16.0,
                                ),
                                TextBox::new(controller.as_controller())
                                    .header(Text::new("Name"))
                                    .placeholder_text("Enter your name")
                                    .description(Text::new(
                                        "The clear button appears while a nonempty single-line editor is focused.",
                                    ))
                                    .is_enabled(enabled)
                                    .is_read_only(read_only)
                                    .text_changed(changed)
                                    .into_widget(),
                                label(
                                    format!("User edits: {edits}"),
                                    TextBlockStyle::Caption,
                                    r.text_fill_color_secondary,
                                ),
                                TextBox::new(controllers[1].as_controller())
                                    .header(Text::new("Read-only sample"))
                                    .is_read_only(true)
                                    .into_widget(),
                            ],
                            16.0,
                        ),
                    ),
                    example(
                        "Multiline text",
                        "Enter line breaks; long lines wrap and the editor scrolls after four visible lines.",
                        &r,
                        TextBox::new(controllers[2].as_controller())
                            .header(Text::new("Notes"))
                            .placeholder_text("Write a few lines…")
                            .accepts_return(true)
                            .max_lines(4)
                            .into_widget(),
                    ),
                ],
                20.0,
            )
        }
    }
}

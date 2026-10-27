//! Command buttons, results, initial focus and cancelable closing of ContentDialog.

use crate::{column, example, label, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the dialog examples.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("ContentDialog", resources, ContentDialogDemo.into_widget())
}

/// Interactive dialog examples.
#[derive(Debug)]
struct ContentDialogDemo;

/// The two retained dialogs and what the last one closed with.
struct ContentDialogDemoState {
    /// Native widget state.
    state: StateData<ContentDialogDemo>,

    /// Save, don't save or cancel.
    save: Option<Handle<ContentDialog>>,

    /// A name to create, which the primary button refuses while it is empty.
    create: Option<Handle<ContentDialog>>,

    /// The create dialog's name field.
    name: Option<Handle<TextEditingController>>,

    /// Most recent result or refusal.
    event: String,
}

impl StatefulWidget for ContentDialogDemo {
    type State = ContentDialogDemoState;

    fn create_state(&self) -> Self::State {
        ContentDialogDemoState {
            state: StateData::new(),
            save: None,
            create: None,
            name: None,
            event: "No dialog shown yet".to_owned(),
        }
    }
}

impl State for ContentDialogDemoState {
    type Widget = ContentDialogDemo;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let save = ContentDialog::new(app)
            .title(app, Text::new("Save your work?"))
            .content(
                app,
                Text::new("Your changes are lost unless you save them before closing."),
            )
            .primary_button_text(app, "Save")
            .secondary_button_text(app, "Don't save")
            .close_button_text(app, "Cancel")
            .default_button(app, ContentDialogButton::Primary);
        let name = TextEditingController::new(app);
        let create = ContentDialog::new(app)
            .title(app, Text::new("New collection"))
            .content(
                app,
                TextBox::new(name.as_controller())
                    .header(Text::new("Name"))
                    .placeholder_text("Photos"),
            )
            .primary_button_text(app, "Create")
            .close_button_text(app, "Cancel")
            .default_button(app, ContentDialogButton::Primary)
            .closing(app, move |app, args| {
                args.set_cancel(
                    args.result() == ContentDialogResult::Primary
                        && name.text_value(app).is_empty(),
                );
                if args.cancel() {
                    self.set_state(app, |state| {
                        state.event = "A collection needs a name".into()
                    });
                }
            });
        for dialog in [save, create] {
            dialog.closed(app, move |app, args| {
                self.set_state(app, |state| {
                    state.event = format!("Closed: {:?}", args.result)
                });
            });
        }
        let state = app.get_mut(self);
        state.save = Some(save);
        state.create = Some(create);
        state.name = Some(name);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let dialogs = [state.save.take(), state.create.take()];
        let name = state.name.take();
        for dialog in dialogs.into_iter().flatten() {
            dialog.dispose(app);
        }
        if let Some(name) = name {
            app.get_mut(name).dispose();
            app.destroy(name);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let (save, create, event) = {
            let state = app.get(self);
            (
                state.save.unwrap(),
                state.create.unwrap(),
                state.event.clone(),
            )
        };
        let text = resources.text_fill_color_secondary;
        column(
            vec![
                example(
                    "Buttons and result",
                    "Primary is the default button: Enter presses it. Escape presses Cancel.",
                    &resources,
                    Button::text(
                        "Show dialog",
                        Listener::new(move |app| self.show(app, save)),
                    )
                    .into_widget(),
                ),
                example(
                    "Initial focus and cancelable closing",
                    "The name field takes the focus. Create refuses to close while the name is empty.",
                    &resources,
                    Button::text(
                        "Create a collection",
                        Listener::new(move |app| self.show(app, create)),
                    )
                    .into_widget(),
                ),
                label(event, TextBlockStyle::Caption, text),
            ],
            24.0,
        )
    }
}

impl ContentDialogDemoState {
    /// Shows `dialog` over the gallery's window.
    fn show(self: Handle<Self>, app: &mut App, dialog: Handle<ContentDialog>) {
        let context = self.context(app);
        if dialog.show_async(app, context).is_err() {
            self.set_state(app, |state| state.event = "A dialog is already open".into());
        }
    }
}

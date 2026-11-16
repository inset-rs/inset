//! Command buttons, results, initial focus and cancelable closing of ContentDialog.

use crate::{column, example, label, section};
use waft_foundation::{App, Handle, Listener, Ref};
use waft_widgets::*;
use waft_winui::*;

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
    save: Option<Ref<ContentDialog>>,

    /// A name to create, which the primary button refuses while it is empty.
    create: Option<Ref<ContentDialog>>,

    /// The create dialog's name field.
    name: Option<Ref<TextEditingController>>,

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
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let save = ContentDialog::new(app);
        save.title(app, Text::new("Save your work?"));
        save.content(
            app,
            Text::new("Your changes are lost unless you save them before closing."),
        );
        save.primary_button_text(app, "Save");
        save.secondary_button_text(app, "Don't save");
        save.close_button_text(app, "Cancel");
        save.default_button(app, ContentDialogButton::Primary);
        let name = TextEditingController::new(app);
        let closing_name = name.clone();
        let create = ContentDialog::new(app);
        create.title(app, Text::new("New collection"));
        create.content(
            app,
            TextBox::new(name.erase())
                .header(Text::new("Name"))
                .placeholder_text("Photos"),
        );
        create.primary_button_text(app, "Create");
        create.close_button_text(app, "Cancel");
        create.default_button(app, ContentDialogButton::Primary);
        create.closing(app, move |app, args| {
            args.set_cancel(
                args.result() == ContentDialogResult::Primary
                    && closing_name.text_value(app).is_empty(),
            );
            if args.cancel() {
                self.set_state(app, |state| {
                    state.event = "A collection needs a name".into()
                });
            }
        });
        for dialog in [save.clone(), create.clone()] {
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
            name.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let (save, create, event) = {
            let state = app.get(self);
            (
                state.save.clone().unwrap(),
                state.create.clone().unwrap(),
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
                        Listener::new({
                            let save = save.clone();
                            move |app| self.show(app, save.handle())
                        }),
                    )
                    .into_widget(),
                ),
                example(
                    "Initial focus and cancelable closing",
                    "The name field takes the focus. Create refuses to close while the name is empty.",
                    &resources,
                    Button::text(
                        "Create a collection",
                        Listener::new({
                            let create = create.clone();
                            move |app| self.show(app, create.handle())
                        }),
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

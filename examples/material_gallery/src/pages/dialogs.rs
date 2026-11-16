//! Dialogs: an `AlertDialog` whose actions pop with a result, a `SimpleDialog` of options, and
//! a full-screen `Dialog`; the caption under the buttons says what came back.
//!
//! Dart writes `await showDialog<T>(..)`; here the call returns the future and the
//! continuation spawned on it records the answer.

use std::any::Any;
use std::rc::Rc;

use waft_foundation::{App, Handle, Listener};
use waft_material::{
    AlertDialog, AppBar, Dialog, FilledButton, Scaffold, SimpleDialog, SimpleDialogOption,
    TextButton, show_dialog,
};
use waft_widgets::{
    BuildContext, IntoWidget, ListView, Navigator, PreferredSizeWidgetRef, State, StateData,
    StatefulWidget, Text, WidgetBuilder, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Dialogs.title(), DialogsDemo).into_widget()
}

#[derive(Debug)]
pub struct DialogsDemo;

/// Dart would spell this `_DialogsDemoState`.
pub struct DialogsDemoState {
    state: StateData<DialogsDemo>,
    answer: Option<String>,
}

impl StatefulWidget for DialogsDemo {
    type State = DialogsDemoState;

    fn create_state(&self) -> DialogsDemoState {
        DialogsDemoState {
            state: StateData::new(),
            answer: None,
        }
    }
}

impl State for DialogsDemoState {
    type Widget = DialogsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let answer = app
            .get(self)
            .answer
            .clone()
            .unwrap_or_else(|| "\u{2014}".to_owned());
        ListView::new()
            .children(vec![
                caption(app, context, format!("Answer: {answer}")),
                section_header(app, context, "Dialogs"),
                showcase(vec![
                    ask_button(self, context, "Alert dialog", alert_dialog()),
                    ask_button(self, context, "Simple dialog", simple_dialog()),
                    ask_button(self, context, "Full-screen dialog", fullscreen_dialog()),
                ]),
            ])
            .into_widget()
    }
}

/// A button that shows `dialog` and records what it pops with, as text.
fn ask_button(
    demo: Handle<DialogsDemoState>,
    context: BuildContext,
    label: &'static str,
    dialog: (WidgetBuilder, bool),
) -> WidgetRef {
    let (builder, fullscreen) = dialog;
    FilledButton::tonal(
        Some(Listener::new(move |app: &mut App| {
            let answer = show_dialog(
                app,
                context,
                Rc::clone(&builder),
                true,
                None,
                None,
                true,
                false,
                None,
                None,
                fullscreen,
                None,
                None,
            );
            answer.then(app, move |app, result| {
                let text = result
                    .and_then(|value| value.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "dismissed".to_owned());
                demo.set_state(app, |state| state.answer = Some(text));
            });
        })),
        Some(Text::new(label)),
    )
    .into_widget()
}

fn pop_with(context: BuildContext, answer: &str) -> Listener {
    let answer: Rc<dyn Any> = Rc::new(answer.to_owned());
    Listener::new(move |app: &mut App| {
        Navigator::pop(app, context, Some(Rc::clone(&answer)));
    })
}

fn alert_dialog() -> (WidgetBuilder, bool) {
    let builder: WidgetBuilder = Rc::new(|_app: &mut App, context: BuildContext| {
        AlertDialog::new()
            .title(Text::new("Discard draft?"))
            .content(Text::new("Your changes will be lost."))
            .actions([
                TextButton::new(Some(pop_with(context, "Cancel")), Some(Text::new("Cancel")))
                    .into_widget(),
                TextButton::new(
                    Some(pop_with(context, "Discard")),
                    Some(Text::new("Discard")),
                )
                .into_widget(),
            ])
            .into_widget()
    });
    (builder, false)
}

fn simple_dialog() -> (WidgetBuilder, bool) {
    let builder: WidgetBuilder = Rc::new(|_app: &mut App, context: BuildContext| {
        let option = |label: &'static str| {
            SimpleDialogOption::new()
                .on_pressed(pop_with(context, label))
                .child(Text::new(label))
                .into_widget()
        };
        SimpleDialog::new()
            .title(Text::new("Select an account"))
            .children([option("Personal"), option("Work")])
            .into_widget()
    });
    (builder, false)
}

fn fullscreen_dialog() -> (WidgetBuilder, bool) {
    let builder: WidgetBuilder = Rc::new(|_app: &mut App, context: BuildContext| {
        Dialog::fullscreen()
            .child(
                Scaffold::new()
                    .app_bar(PreferredSizeWidgetRef::new(
                        AppBar::new().title(Text::new("New event")).actions(vec![
                            TextButton::new(
                                Some(pop_with(context, "Saved")),
                                Some(Text::new("Save")),
                            )
                            .into_widget(),
                        ]),
                    ))
                    .body(Text::new("A full-screen dialog")),
            )
            .into_widget()
    });
    (builder, true)
}

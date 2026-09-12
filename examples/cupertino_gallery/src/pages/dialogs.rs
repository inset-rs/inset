//! Dialogs, action sheets, modal popups and sheets — and the result each one hands back.
//!
//! Dart writes `if (await showCupertinoDialog<bool>(...))`. Here the call runs inline and hands
//! back the future; what follows Dart's `await` is the spawned continuation.

use std::any::Any;
use std::rc::Rc;

use inset_cupertino::{
    CupertinoActionSheet, CupertinoActionSheetAction, CupertinoAlertDialog, CupertinoButton,
    CupertinoDialogAction, CupertinoNavigationBar, CupertinoPopupSurface,
    K_CUPERTINO_MODAL_BARRIER_COLOR, show_cupertino_dialog, show_cupertino_modal_popup,
    show_cupertino_sheet,
};
use inset_foundation::{App, Handle, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize};
use inset_widgets::{
    BuildContext, Column, IntoWidget, Navigator, Padding, Row, State, StateData, StatefulWidget,
    Text, WidgetBuilder, WidgetRef,
};

use crate::support::{
    body_style, screen, screen_with_bar, scrolling_body, secondary, section, section_with_footer,
    showcase,
};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    DialogsDemo.into_widget()
}

#[derive(Debug)]
pub struct DialogsDemo;

/// Dart would spell this `_DialogsDemoState`.
pub struct LastAnswer {
    state: StateData<DialogsDemo>,
    text: Option<String>,
}

impl StatefulWidget for DialogsDemo {
    type State = LastAnswer;

    fn create_state(&self) -> LastAnswer {
        LastAnswer {
            state: StateData::new(),
            text: None,
        }
    }
}

impl State for LastAnswer {
    type Widget = DialogsDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let answer = app.get(self).text.clone();

        screen(scrolling_body(vec![
            section_with_footer(
                "Alert dialog",
                "The barrier is not dismissible: each action pops with its own result.",
                vec![
                    showcase(CupertinoButton::filled(
                        Text::new("Ask a question\u{2026}").into_widget(),
                        Some(ask(self, context, confirm_dialog, |yes| {
                            if yes { "Yes" } else { "No" }
                        })),
                    )),
                    showcase(CupertinoButton::filled(
                        Text::new("Show a destructive choice\u{2026}").into_widget(),
                        Some(ask(self, context, delete_dialog, |yes| {
                            if yes { "Deleted" } else { "Kept" }
                        })),
                    )),
                ],
            ),
            section_with_footer(
                "Action sheet",
                "This barrier IS dismissible — tap outside and no result comes back.",
                vec![showcase(CupertinoButton::filled(
                    Text::new("Show an action sheet\u{2026}").into_widget(),
                    Some(action_sheet(self, context)),
                ))],
            ),
            section_with_footer(
                "Modal popup",
                "CupertinoPopupSurface is the frosted card the alert dialog above is built on. \
                 An action sheet is not — it clips and blurs its own.",
                vec![showcase(CupertinoButton::new(
                    Text::new("Show a bare popup\u{2026}").into_widget(),
                    Some(bare_popup(context)),
                ))],
            ),
            section_with_footer(
                "Sheet",
                "A whole page, inset from the top, over a card that shrinks back behind it.",
                vec![showcase(CupertinoButton::new(
                    Text::new("Show a sheet\u{2026}").into_widget(),
                    Some(sheet(self, context)),
                ))],
            ),
            section(
                "Result",
                vec![
                    Padding::new(EdgeInsetsGeometry::symmetric(12.0, 16.0))
                        .child(
                            Row::new()
                                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                                .children([
                                    Text::new("Last answer")
                                        .style(body_style(app, context))
                                        .into_widget(),
                                    secondary(
                                        app,
                                        context,
                                        answer.unwrap_or_else(|| "\u{2014}".to_owned()),
                                    ),
                                ]),
                        )
                        .into_widget(),
                ],
            ),
        ]))
    }
}

/// Opens a dialog and reports its `bool` result to the demo's state.
///
/// `dialog` and `describe` are `fn` pointers rather than closures: the only state that has to
/// travel is the state handle and the context, and both are `Copy`.
fn ask(
    this: Handle<LastAnswer>,
    context: BuildContext,
    dialog: fn(&mut App, BuildContext) -> WidgetRef,
    describe: fn(bool) -> &'static str,
) -> Listener {
    Listener::new(move |app: &mut App| {
        let builder: WidgetBuilder = Rc::new(dialog);
        let answer =
            show_cupertino_dialog(app, context, builder, None, None, true, false, None, None);
        app.spawn(async move |cx| {
            let answer = answer
                .await
                .and_then(|value| value.downcast_ref::<bool>().copied())
                .map(|yes| describe(yes).to_owned());
            cx.update(|app| record(this, app, answer));
        });
    })
}

fn action_sheet(this: Handle<LastAnswer>, context: BuildContext) -> Listener {
    Listener::new(move |app: &mut App| {
        let builder: WidgetBuilder = Rc::new(sheet_actions);
        let chosen = show_cupertino_modal_popup(
            app,
            context,
            builder,
            None,
            K_CUPERTINO_MODAL_BARRIER_COLOR,
            true,
            true,
            false,
            None,
            None,
        );
        app.spawn(async move |cx| {
            // `None` here is the dismissible barrier: tapped outside, the route popped with no
            // result at all.
            let chosen = chosen
                .await
                .and_then(|value| value.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "Dismissed".to_owned());
            cx.update(|app| record(this, app, Some(chosen)));
        });
    })
}

fn bare_popup(context: BuildContext) -> Listener {
    Listener::new(move |app: &mut App| {
        let builder: WidgetBuilder = Rc::new(popup_card);
        show_cupertino_modal_popup(
            app,
            context,
            builder,
            None,
            K_CUPERTINO_MODAL_BARRIER_COLOR,
            true,
            true,
            false,
            None,
            None,
        );
    })
}

fn sheet(this: Handle<LastAnswer>, context: BuildContext) -> Listener {
    Listener::new(move |app: &mut App| {
        let builder: WidgetBuilder = Rc::new(sheet_page);
        let closed = show_cupertino_sheet(
            app,
            context,
            Some(builder),
            None,
            false,
            true,
            None,
            None,
            false,
        );
        app.spawn(async move |cx| {
            let closed = closed
                .await
                .and_then(|value| value.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "Dragged down".to_owned());
            cx.update(|app| record(this, app, Some(closed)));
        });
    })
}

fn record(this: Handle<LastAnswer>, app: &mut App, answer: Option<String>) {
    if this.mounted(app) {
        this.set_state(app, |state| state.text = answer);
    }
}

fn confirm_dialog(_app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoAlertDialog::new()
        .title(Text::new("Proceed?"))
        .content(Text::new("The thing will be done."))
        .actions([
            CupertinoDialogAction::new(Text::new("Cancel"))
                .on_pressed(pop_with(context, false))
                .into_widget(),
            CupertinoDialogAction::new(Text::new("OK"))
                .is_default_action(true)
                .on_pressed(pop_with(context, true))
                .into_widget(),
        ])
        .into_widget()
}

fn delete_dialog(_app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoAlertDialog::new()
        .title(Text::new("Delete this?"))
        .content(Text::new("This cannot be undone."))
        .actions([
            CupertinoDialogAction::new(Text::new("Keep"))
                .is_default_action(true)
                .on_pressed(pop_with(context, false))
                .into_widget(),
            CupertinoDialogAction::new(Text::new("Delete"))
                .is_destructive_action(true)
                .on_pressed(pop_with(context, true))
                .into_widget(),
        ])
        .into_widget()
}

fn sheet_actions(_app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoActionSheet::new()
        .title(Text::new("Save changes?"))
        .message(Text::new("Your edits have not been written yet."))
        .actions([
            CupertinoActionSheetAction::new(
                pop_with(context, "Save".to_owned()),
                Text::new("Save"),
            )
            .is_default_action(true)
            .into_widget(),
            CupertinoActionSheetAction::new(
                pop_with(context, "Discard".to_owned()),
                Text::new("Discard"),
            )
            .is_destructive_action(true)
            .into_widget(),
        ])
        .cancel_button(CupertinoActionSheetAction::new(
            pop_with(context, "Cancel".to_owned()),
            Text::new("Cancel"),
        ))
        .into_widget()
}

fn popup_card(app: &mut App, context: BuildContext) -> WidgetRef {
    let style = body_style(app, context);
    Padding::new(EdgeInsetsGeometry::all(8.0))
        .child(CupertinoPopupSurface::new(
            Padding::new(EdgeInsetsGeometry::all(24.0)).child(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .children([
                        Text::new("A popup surface")
                            .style(style.clone())
                            .into_widget(),
                        Padding::new(EdgeInsetsGeometry::from_ltrb(0.0, 8.0, 0.0, 0.0))
                            .child(Text::new("Tap outside to dismiss.").style(style))
                            .into_widget(),
                    ]),
            ),
        ))
        .into_widget()
}

/// The sheet's own page: a scaffold with a bar, so the sheet reads as a screen rather than a
/// card, which is what `CupertinoSheetRoute` is for.
fn sheet_page(app: &mut App, context: BuildContext) -> WidgetRef {
    screen_with_bar(
        CupertinoNavigationBar::new()
            .middle(Text::new("A Sheet"))
            .trailing(CupertinoButton::new(
                Text::new("Done").into_widget(),
                Some(pop_with(context, "Done".to_owned())),
            ))
            .automatically_imply_leading(false),
        scrolling_body(vec![section_with_footer(
            "Cupertino sheet",
            "Drag it down from the top to dismiss it; the page behind shrinks back into place.",
            vec![showcase(secondary(
                app,
                context,
                "show_cupertino_sheet pushes to the root navigator.",
            ))],
        )]),
    )
}

fn pop_with<T: 'static>(context: BuildContext, result: T) -> Listener {
    let result: Rc<dyn Any> = Rc::new(result);
    Listener::new(move |app: &mut App| {
        Navigator::pop(app, context, Some(Rc::clone(&result)));
    })
}

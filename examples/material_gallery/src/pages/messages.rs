//! Messages and sheets: `SnackBar`s fixed, floating, with an action and with a close icon, a
//! `MaterialBanner`, both shown through the app's `ScaffoldMessenger`; a modal and a
//! persistent bottom sheet; and `Tooltip`s with a plain and a rich message, shown by hover or
//! long press, by a tap, or above their child.
//!
//! The messenger belongs to the `MaterialApp`, not to this page, so a snack bar or a banner
//! outlives the page that showed it and stays up on the index after the back button. That is
//! why the callbacks here hold the messenger rather than this page's context.

use std::rc::Rc;

use waft_foundation::{App, Listener};
use waft_material::{
    FilledButton, Icons, MaterialBanner, MaterialBannerClosedReason, OutlinedButton,
    ScaffoldMessenger, ShowBottomSheet, ShowModalBottomSheet, SnackBar, SnackBarAction,
    SnackBarBehavior, TextButton, Tooltip,
};
use waft_painting::{EdgeInsetsGeometry, TextSpan};
use waft_rendering::MainAxisSize;
use waft_widgets::{
    BuildContext, Builder, Column, Icon, IntoWidget, ListView, Navigator, Padding, SizedBox, Text,
    TooltipTriggerMode, WidgetBuilder, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{page, section_header, showcase};

const MODAL_SHEET_TEXT: &str = "A modal bottom sheet";
pub const PERSISTENT_SHEET_TEXT: &str = "A persistent bottom sheet";
pub const BANNER_TEXT: &str = "A material banner";

/// Flutter's `kDefaultScrollControlDisabledMaxHeightRatio`: the modal sheet's height cap when
/// it is not scroll-controlled.
pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    // The body's own context, under the page's scaffold: the persistent sheet is shown in
    // the nearest scaffold, which the route's context is above.
    let content = Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Snack bars"),
                showcase(snack_bar_buttons(context)),
                section_header(app, context, "Banner"),
                showcase(vec![show_banner_button(context)]),
                section_header(app, context, "Bottom sheets"),
                showcase(vec![
                    modal_sheet_button(context),
                    persistent_sheet_button(context),
                ]),
                section_header(app, context, "Tooltips"),
                showcase(tooltips()),
            ])
            .into_widget()
    });
    page(Entry::Messages.title(), content).into_widget()
}

fn snack_bar_buttons(context: BuildContext) -> Vec<WidgetRef> {
    vec![
        show_snack_bar_button(context, "Fixed", || {
            SnackBar::new(Text::new("A fixed snack bar"))
        }),
        show_snack_bar_button(context, "Floating", || {
            SnackBar::new(Text::new("A floating snack bar")).behavior(SnackBarBehavior::Floating)
        }),
        show_snack_bar_with_action_button(context),
        show_snack_bar_button(context, "With close icon", || {
            SnackBar::new(Text::new("A snack bar with a close icon"))
                .behavior(SnackBarBehavior::Floating)
                .show_close_icon(true)
        }),
    ]
}

/// A button labelled `label` that shows the snack bar `snack_bar` makes.
fn show_snack_bar_button(
    context: BuildContext,
    label: &str,
    snack_bar: impl Fn() -> SnackBar + 'static,
) -> WidgetRef {
    FilledButton::tonal(
        Some(Listener::new(move |app: &mut App| {
            ScaffoldMessenger::of(app, context).show_snack_bar(app, snack_bar(), None);
        })),
        Some(Text::new(label)),
    )
    .into_widget()
}

/// A snack bar whose action answers with a second one, queued behind it.
fn show_snack_bar_with_action_button(context: BuildContext) -> WidgetRef {
    FilledButton::tonal(
        Some(Listener::new(move |app: &mut App| {
            let messenger = ScaffoldMessenger::of(app, context);
            let undo = SnackBarAction::new("Undo", move |app: &mut App| {
                messenger.show_snack_bar(app, SnackBar::new(Text::new("Undone")), None);
            });
            messenger.show_snack_bar(
                app,
                SnackBar::new(Text::new("Message archived")).action(undo),
                None,
            );
        })),
        Some(Text::new("With action")),
    )
    .into_widget()
}

fn show_banner_button(context: BuildContext) -> WidgetRef {
    FilledButton::tonal(
        Some(Listener::new(move |app: &mut App| {
            let messenger = ScaffoldMessenger::of(app, context);
            let dismiss = TextButton::new(
                Some(Listener::new(move |app: &mut App| {
                    messenger.hide_current_material_banner(app, MaterialBannerClosedReason::Hide);
                })),
                Some(Text::new("Dismiss")),
            );
            messenger.show_material_banner(
                app,
                MaterialBanner::new(Text::new(BANNER_TEXT), vec![dismiss.into_widget()])
                    .leading(Icon::new(Some(Icons::info()))),
            );
        })),
        Some(Text::new("Show banner")),
    )
    .into_widget()
}

fn modal_sheet_button(context: BuildContext) -> WidgetRef {
    FilledButton::new(
        Some(Listener::new(move |app: &mut App| {
            // Dart's defaults for every argument but the builder.
            ShowModalBottomSheet::new(sheet(MODAL_SHEET_TEXT, "Close sheet")).show(app, context);
        })),
        Some(Text::new("Modal sheet")),
    )
    .into_widget()
}

fn persistent_sheet_button(context: BuildContext) -> WidgetRef {
    OutlinedButton::new(
        Some(Listener::new(move |app: &mut App| {
            ShowBottomSheet::new(sheet(PERSISTENT_SHEET_TEXT, "Close persistent sheet"))
                .show(app, context);
        })),
        Some(Text::new("Persistent sheet")),
    )
    .into_widget()
}

/// A sheet reading `text` over a button that closes it.
///
/// `Navigator::pop` closes either kind: a modal sheet is a route, and a persistent sheet is a
/// local history entry of the page's route.
fn sheet(text: &'static str, close_label: &'static str) -> WidgetBuilder {
    Rc::new(move |_app: &mut App, context: BuildContext| {
        SizedBox::new()
            .width(f64::INFINITY)
            .child(
                Padding::new(EdgeInsetsGeometry::all(32.0)).child(
                    Column::new().main_axis_size(MainAxisSize::Min).children([
                        Text::new(text).into_widget(),
                        SizedBox::new().height(16.0).into_widget(),
                        FilledButton::new(
                            Some(Listener::new(move |app: &mut App| {
                                Navigator::pop(app, context, None);
                            })),
                            Some(Text::new(close_label)),
                        )
                        .into_widget(),
                    ]),
                ),
            )
            .into_widget()
    })
}

/// A tooltip on a button of each way a tooltip shows: a hover or long press, a tap, below or
/// above the child, and with a rich message.
fn tooltips() -> Vec<WidgetRef> {
    let button = |label: &'static str| {
        TextButton::new(
            Some(Listener::new(|_app: &mut App| {})),
            Some(Text::new(label)),
        )
    };
    vec![
        Tooltip::new()
            .message("Shown by hover or long press")
            .child(button("Hover or hold"))
            .into_widget(),
        Tooltip::new()
            .message("Shown by a tap")
            .trigger_mode(TooltipTriggerMode::Tap)
            .child(button("Tap"))
            .into_widget(),
        Tooltip::new()
            .message("Shown above its child")
            .prefer_below(false)
            .child(button("Above"))
            .into_widget(),
        Tooltip::new()
            .rich_message(Rc::new(TextSpan::new().text("A rich message")))
            .child(button("Rich"))
            .into_widget(),
    ]
}

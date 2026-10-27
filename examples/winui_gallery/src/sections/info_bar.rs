//! Severity, actions and cancelable InfoBar closing.

use crate::{column, example, example_row, section};
use inset_foundation::{App, Handle, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_widgets::*;
use inset_winui::*;

/// Builds the notification examples.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("InfoBar", resources, InfoBarDemo.into_widget())
}

/// Interactive notification examples.
#[derive(Debug)]
struct InfoBarDemo;

/// Owner values for the dismissible notification.
struct InfoBarDemoState {
    /// Native widget state.
    state: StateData<InfoBarDemo>,

    /// Whether the interactive banner is open.
    open: bool,

    /// Cancels either way of closing the banner.
    cancel: bool,

    /// Most recent lifecycle event.
    event: String,
}

impl StatefulWidget for InfoBarDemo {
    type State = InfoBarDemoState;

    fn create_state(&self) -> Self::State {
        InfoBarDemoState {
            state: StateData::new(),
            open: true,
            cancel: false,
            event: "No close request yet".to_owned(),
        }
    }
}

impl State for InfoBarDemoState {
    type Widget = InfoBarDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let (open, cancel, event) = {
            let state = app.get(self);
            (state.open, state.cancel, state.event.clone())
        };
        let severity_examples = [
            (
                InfoBarSeverity::Informational,
                "Information",
                "An update is available.",
            ),
            (
                InfoBarSeverity::Success,
                "Success",
                "Your changes have been saved.",
            ),
            (
                InfoBarSeverity::Warning,
                "Warning",
                "Storage is almost full.",
            ),
            (
                InfoBarSeverity::Error,
                "Error",
                "The download could not be completed.",
            ),
        ]
        .into_iter()
        .map(|(severity, title, message)| {
            stretched(
                InfoBar::new(true, |_, _| {})
                    .severity(severity)
                    .title(title)
                    .message(message)
                    .is_closable(false),
            )
        })
        .collect();

        column(
            vec![
                example(
                    "Severity",
                    "Each severity supplies its own background and status icon.",
                    &resources,
                    column(severity_examples, 12.0),
                ),
                example(
                    "Closing and cancellation",
                    "Use the close button or close from the owner. Cancel closing keeps the banner open.",
                    &resources,
                    column(
                        vec![
                            example_row(
                                vec![
                                    Button::text(
                                        "Open notification",
                                        Listener::new(move |app| {
                                            self.set_state(app, |state| state.open = true);
                                        }),
                                    )
                                    .into_widget(),
                                    Button::text(
                                        "Close notification",
                                        Listener::new(move |app| {
                                            self.set_state(app, |state| state.open = false);
                                        }),
                                    )
                                    .into_widget(),
                                    CheckBox::new(Some(cancel), move |app, checked| {
                                        self.set_state(app, |state| {
                                            state.cancel = checked == Some(true)
                                        });
                                    })
                                    .content(Text::new("Cancel closing"))
                                    .into_widget(),
                                ],
                                12.0,
                            ),
                            stretched(
                                InfoBar::new(open, move |app, open| {
                                    self.set_state(app, |state| state.open = open);
                                })
                                .title("Download ready")
                                .message("The file is ready to open.")
                                .closing(move |app, args| {
                                    args.cancel = app.get(self).cancel;
                                    self.set_state(app, |state| {
                                        state.event = format!("Closing: {:?}", args.reason)
                                    });
                                })
                                .closed(move |app, args| {
                                    self.set_state(app, |state| {
                                        state.event = format!("Closed: {:?}", args.reason)
                                    });
                                })
                                .opened(Listener::new(
                                    move |app| {
                                        self.set_state(app, |state| {
                                            state.event = "Opened".to_owned()
                                        });
                                    },
                                )),
                            ),
                            Text::new(event).into_widget(),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Action and extra content",
                    "The title, message and action stack when space is limited. Extra content supplies its own spacing.",
                    &resources,
                    stretched(
                        InfoBar::new(true, |_, _| {})
                            .title("Sync paused")
                            .message("Review your connection before continuing.")
                            .severity(InfoBarSeverity::Warning)
                            .is_closable(false)
                            .action_button(HyperlinkButton::text(
                                "Resume sync",
                                Listener::new(move |app| {
                                    self.set_state(app, |state| {
                                        state.event = "Resume requested".to_owned()
                                    });
                                }),
                            ))
                            .content(
                                Padding::new(EdgeInsetsGeometry::from_ltrb(0.0, 0.0, 16.0, 16.0))
                                    .child(Text::new("Your local changes are preserved.")),
                            ),
                    ),
                ),
                example(
                    "Custom icon",
                    "A custom icon can accompany a notification; the next example hides the icon entirely.",
                    &resources,
                    column(
                        vec![
                            stretched(
                                InfoBar::new(true, |_, _| {})
                                    .title("Scheduled update")
                                    .icon_source(FontIcon::symbol(FluentSymbol::Timer))
                                    .is_closable(false),
                            ),
                            stretched(
                                InfoBar::new(true, |_, _| {})
                                    .message("A quiet notification without an icon.")
                                    .is_icon_visible(false)
                                    .is_closable(false),
                            ),
                        ],
                        12.0,
                    ),
                ),
            ],
            16.0,
        )
    }
}

/// `bar` across its example's width, where WinUI's default `HorizontalAlignment` of Stretch
/// places it.
fn stretched(bar: InfoBar) -> WidgetRef {
    SizedBox::new()
        .width(f64::INFINITY)
        .child(bar)
        .into_widget()
}

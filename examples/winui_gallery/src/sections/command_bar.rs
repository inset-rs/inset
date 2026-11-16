//! CommandBar: a bar of commands with labels under their icons and an overflow menu, the label positions and closed display modes, and a bar whose commands move into the overflow as it narrows.

use crate::{column, example, label, row, section};
use waft_foundation::{App, Handle, Listener};
use waft_widgets::*;
use waft_winui::*;

/// Builds the feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("CommandBar", resources, CommandBarDemo.into_widget())
}

/// A glyph of the bundled Fluent System Icons font, by its code point.
fn glyph(code_point: u32) -> FontIcon {
    FontIcon::new(char::from_u32(code_point).unwrap().to_string())
}

const EDIT: u32 = 62429;
const SHARE: u32 = 63151;
const DELETE: u32 = 62284;
const SAVE: u32 = 63103;
const COPY: u32 = 62251;
const CUT: u32 = 62266;
const PASTE: u32 = 62165;
const BOLD: u32 = 63396;
const ITALIC: u32 = 63476;
const UNDERLINE: u32 = 63498;
const FILTER: u32 = 62470;
const SYNC: u32 = 61840;
const PIN: u32 = 62977;

/// The page's bars and what the application keeps for them.
#[derive(Debug)]
struct CommandBarDemo;

struct CommandBarDemoState {
    state: StateData<CommandBarDemo>,
    /// Which bar is open, by name.
    open: Option<&'static str>,
    bold: bool,
    italic: bool,
    pinned: bool,
    label_position: CommandBarDefaultLabelPosition,
    closed_display_mode: AppBarClosedDisplayMode,
    sticky: bool,
    /// The narrowing bar's width.
    width: f64,
    last: String,
    events: Vec<String>,
}

impl StatefulWidget for CommandBarDemo {
    type State = CommandBarDemoState;

    fn create_state(&self) -> Self::State {
        CommandBarDemoState {
            state: StateData::new(),
            open: None,
            bold: false,
            italic: false,
            pinned: false,
            label_position: CommandBarDefaultLabelPosition::Bottom,
            closed_display_mode: AppBarClosedDisplayMode::Compact,
            sticky: false,
            width: 640.0,
            last: "None".into(),
            events: Vec::new(),
        }
    }
}

impl CommandBarDemoState {
    fn command(self: Handle<Self>, label: &'static str, code_point: u32) -> AppBarButton {
        AppBarButton::new(
            label,
            Listener::new(move |app| self.set_state(app, |s| s.last = label.into())),
        )
        .icon(glyph(code_point))
    }

    fn log(self: Handle<Self>, event: &'static str) -> Listener {
        Listener::new(move |app| {
            self.set_state(app, |s| {
                s.events.push(event.to_string());
                let excess = s.events.len().saturating_sub(4);
                s.events.drain(..excess);
            })
        })
    }

    /// A bar named `name`, open while the page says so.
    fn bar(self: Handle<Self>, app: &App, name: &'static str) -> CommandBar {
        CommandBar::new(app.get(self).open == Some(name), move |app, open| {
            self.set_state(app, |s| s.open = open.then_some(name));
        })
    }

    fn editing_bar(self: Handle<Self>, app: &App, resources: &ThemeResources) -> WidgetRef {
        let state = app.get(self);
        self.bar(app, "editing")
            .primary_commands(vec![
                self.command("Save", SAVE)
                    .keyboard_accelerator_text_override("Ctrl+S")
                    .into(),
                self.command("Edit", EDIT).into(),
                self.command("Share", SHARE).into(),
                AppBarSeparator::new().into(),
                AppBarToggleButton::new("Bold", Some(state.bold), move |app, value| {
                    self.set_state(app, |s| s.bold = value == Some(true))
                })
                .icon(glyph(BOLD))
                .into(),
                AppBarToggleButton::new("Italic", Some(state.italic), move |app, value| {
                    self.set_state(app, |s| s.italic = value == Some(true))
                })
                .icon(glyph(ITALIC))
                .into(),
            ])
            .secondary_commands(vec![
                self.command("Delete", DELETE)
                    .keyboard_accelerator_text_override("Del")
                    .into(),
                AppBarToggleButton::new("Pin", Some(state.pinned), move |app, value| {
                    self.set_state(app, |s| s.pinned = value == Some(true))
                })
                .icon(glyph(PIN))
                .into(),
            ])
            .content(label(
                "Document.txt",
                TextBlockStyle::Body,
                resources.text_fill_color_primary,
            ))
            .default_label_position(state.label_position)
            .closed_display_mode(state.closed_display_mode)
            .is_sticky(state.sticky)
            .opening(self.log("Opening"))
            .opened(self.log("Opened"))
            .closing(self.log("Closing"))
            .closed(self.log("Closed"))
            .into_widget()
    }

    fn narrowing_bar(self: Handle<Self>, app: &App) -> WidgetRef {
        let width = app.get(self).width;
        let bar = self
            .bar(app, "narrowing")
            .primary_commands(vec![
                self.command("Cut", CUT).into(),
                self.command("Copy", COPY).into(),
                self.command("Paste", PASTE).into(),
                AppBarSeparator::new().into(),
                self.command("Underline", UNDERLINE)
                    .dynamic_overflow_order(1)
                    .into(),
                self.command("Filter", FILTER).into(),
                self.command("Sync", SYNC).into(),
            ])
            .secondary_commands(vec![self.command("Delete", DELETE).into()])
            .dynamic_overflow_items_changing(move |app, args| {
                let event = match args.action {
                    CommandBarDynamicOverflowAction::AddingToOverflow => "Adding to overflow",
                    CommandBarDynamicOverflowAction::RemovingFromOverflow => {
                        "Removing from overflow"
                    }
                };
                self.set_state(app, |s| s.last = event.into());
            });
        SizedBox::new().width(width).child(bar).into_widget()
    }

    fn option(
        self: Handle<Self>,
        text: &'static str,
        change: impl Fn(&mut CommandBarDemoState) + 'static,
    ) -> WidgetRef {
        Button::text(
            text,
            Listener::new(move |app| self.set_state(app, |s| change(s))),
        )
        .into_widget()
    }
}

impl State for CommandBarDemoState {
    type Widget = CommandBarDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let state = app.get(self);
        let (label_position, mode, sticky, width) = (
            state.label_position,
            state.closed_display_mode,
            state.sticky,
            state.width,
        );
        let status = format!(
            "Last command: {}   Bold: {}   Italic: {}   Pinned: {}   Events: {}",
            state.last,
            state.bold,
            state.italic,
            state.pinned,
            if state.events.is_empty() {
                "none".to_string()
            } else {
                state.events.join(", ")
            }
        );
        column(
            vec![
                example(
                    "Commands with an overflow menu",
                    "Open the bar with the more button to show the labels and the secondary commands. Arrow keys move along the bar, Down on the more button moves into the menu, Escape closes it.",
                    &resources,
                    column(
                        vec![
                            self.editing_bar(app, &resources),
                            row(
                                vec![
                                    self.option(
                                        match label_position {
                                            CommandBarDefaultLabelPosition::Bottom => {
                                                "Labels: Bottom"
                                            }
                                            CommandBarDefaultLabelPosition::Right => {
                                                "Labels: Right"
                                            }
                                            CommandBarDefaultLabelPosition::Collapsed => {
                                                "Labels: Collapsed"
                                            }
                                        },
                                        |s| {
                                            s.label_position = match s.label_position {
                                                CommandBarDefaultLabelPosition::Bottom => {
                                                    CommandBarDefaultLabelPosition::Right
                                                }
                                                CommandBarDefaultLabelPosition::Right => {
                                                    CommandBarDefaultLabelPosition::Collapsed
                                                }
                                                CommandBarDefaultLabelPosition::Collapsed => {
                                                    CommandBarDefaultLabelPosition::Bottom
                                                }
                                            }
                                        },
                                    ),
                                    self.option(
                                        match mode {
                                            AppBarClosedDisplayMode::Compact => "Closed: Compact",
                                            AppBarClosedDisplayMode::Minimal => "Closed: Minimal",
                                            AppBarClosedDisplayMode::Hidden => "Closed: Hidden",
                                        },
                                        |s| {
                                            s.closed_display_mode = match s.closed_display_mode {
                                                AppBarClosedDisplayMode::Compact => {
                                                    AppBarClosedDisplayMode::Minimal
                                                }
                                                AppBarClosedDisplayMode::Minimal => {
                                                    AppBarClosedDisplayMode::Hidden
                                                }
                                                AppBarClosedDisplayMode::Hidden => {
                                                    AppBarClosedDisplayMode::Compact
                                                }
                                            }
                                        },
                                    ),
                                    self.option(
                                        if sticky { "Sticky: On" } else { "Sticky: Off" },
                                        |s| s.sticky = !s.sticky,
                                    ),
                                    self.option("Open", |s| s.open = Some("editing")),
                                ],
                                12.0,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Dynamic overflow",
                    "Narrow the bar: the rightmost commands move into the overflow first, Underline before them by its DynamicOverflowOrder, and come back as it widens.",
                    &resources,
                    column(
                        vec![
                            Slider::new(width, move |app, value| {
                                self.set_state(app, |s| s.width = value)
                            })
                            .minimum(120.0)
                            .maximum(640.0)
                            .header(Text::new(format!("Width: {:.0}", width)))
                            .into_widget(),
                            self.narrowing_bar(app),
                        ],
                        12.0,
                    ),
                ),
                label(
                    status,
                    TextBlockStyle::Body,
                    resources.text_fill_color_secondary,
                ),
            ],
            24.0,
        )
    }
}

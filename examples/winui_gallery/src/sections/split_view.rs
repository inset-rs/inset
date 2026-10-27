//! SplitView's four display modes, pane placement, dimming and cancelable dismissal.
use crate::{column, example, example_row, label, section};
use inset_foundation::{App, Handle, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_widgets::*;
use inset_winui::*;

/// Builds the configurable pane layout example.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "SplitView",
        resources,
        example(
            "Pane layout and light dismissal",
            "Compare all four display modes and both pane placements. Overlay modes can dismiss on outside click; inline modes reserve content space.",
            resources,
            SplitViewDemo.into_widget(),
        ),
    )
}

/// Owns the interactive pane configuration.
#[derive(Debug)]
struct SplitViewDemo;

/// Tracks pane options, selected content, and lifecycle events.
struct SplitViewDemoState {
    /// Native widget lifecycle.
    state: StateData<SplitViewDemo>,

    /// Owner-managed open state.
    open: bool,

    /// Current pane layout mode.
    mode: SplitViewDisplayMode,

    /// Side containing the pane.
    placement: SplitViewPanePlacement,

    /// Whether overlay modes dim the content.
    dim: bool,

    /// Whether dismiss requests are canceled.
    cancel: bool,

    /// Selected content label.
    page: &'static str,

    /// Most recent pane lifecycle event.
    event: &'static str,

    /// Uses a wider open pane length.
    wide: bool,
}

impl StatefulWidget for SplitViewDemo {
    type State = SplitViewDemoState;

    fn create_state(&self) -> Self::State {
        SplitViewDemoState {
            state: StateData::new(),
            open: true,
            mode: SplitViewDisplayMode::CompactInline,
            placement: SplitViewPanePlacement::Left,
            dim: true,
            cancel: false,
            page: "Home",
            event: "Ready",
            wide: false,
        }
    }
}

impl State for SplitViewDemoState {
    type Widget = SplitViewDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let text = resources.text_fill_color_primary;
        let (open, mode, placement, dim, cancel, page, event) = {
            let state = app.get(self);
            (
                state.open,
                state.mode,
                state.placement,
                state.dim,
                state.cancel,
                state.page,
                state.event,
            )
        };
        let wide = app.get(self).wide;
        let mode_name = match mode {
            SplitViewDisplayMode::Overlay => "Overlay",
            SplitViewDisplayMode::Inline => "Inline",
            SplitViewDisplayMode::CompactOverlay => "CompactOverlay",
            SplitViewDisplayMode::CompactInline => "CompactInline",
        };
        let controls = example_row(
            vec![
                Button::text(
                    if open { "Close pane" } else { "Open pane" },
                    Listener::new(move |app| self.set_state(app, |s| s.open = !s.open)),
                )
                .into_widget(),
                Button::text(
                    mode_name,
                    Listener::new(move |app| {
                        self.set_state(app, |s| {
                            s.mode = match s.mode {
                                SplitViewDisplayMode::Overlay => SplitViewDisplayMode::Inline,
                                SplitViewDisplayMode::Inline => {
                                    SplitViewDisplayMode::CompactOverlay
                                }
                                SplitViewDisplayMode::CompactOverlay => {
                                    SplitViewDisplayMode::CompactInline
                                }
                                SplitViewDisplayMode::CompactInline => {
                                    SplitViewDisplayMode::Overlay
                                }
                            };
                        })
                    }),
                )
                .into_widget(),
                Button::text(
                    match placement {
                        SplitViewPanePlacement::Left => "Pane: left",
                        SplitViewPanePlacement::Right => "Pane: right",
                    },
                    Listener::new(move |app| {
                        self.set_state(app, |s| {
                            s.placement = match s.placement {
                                SplitViewPanePlacement::Left => SplitViewPanePlacement::Right,
                                SplitViewPanePlacement::Right => SplitViewPanePlacement::Left,
                            };
                        })
                    }),
                )
                .into_widget(),
            ],
            8.0,
        );
        let options = example_row(
            vec![
                CheckBox::new(Some(dim), move |app, value| {
                    self.set_state(app, |s| s.dim = value.unwrap_or(false))
                })
                .content(Text::new("Dim content"))
                .into_widget(),
                CheckBox::new(Some(cancel), move |app, value| {
                    self.set_state(app, |s| s.cancel = value.unwrap_or(false))
                })
                .content(Text::new("Cancel light dismiss"))
                .into_widget(),
                CheckBox::new(Some(wide), move |app, value| {
                    self.set_state(app, |s| s.wide = value.unwrap_or(false))
                })
                .content(Text::new("Wide pane (240 px)"))
                .into_widget(),
            ],
            16.0,
        );
        let pane = Padding::new(EdgeInsetsGeometry::all(8.0)).child(column(
            vec![
                label("Pane", TextBlockStyle::BodyStrong, text),
                Button::text(
                    "Home",
                    Listener::new(move |app| self.set_state(app, |s| s.page = "Home")),
                )
                .into_widget(),
                Button::text(
                    "Settings",
                    Listener::new(move |app| self.set_state(app, |s| s.page = "Settings")),
                )
                .into_widget(),
            ],
            8.0,
        ));
        let content = ColoredBox::new(resources.solid_background_fill_color_secondary).child(
            Center::new().child(column(
                vec![
                    label(page, TextBlockStyle::Subtitle, text),
                    label(
                        "Pane and content keep their state.",
                        TextBlockStyle::Caption,
                        text,
                    ),
                ],
                8.0,
            )),
        );
        let split = SplitView::new(open, move |app, value| {
            self.set_state(app, |s| s.open = value)
        })
        .display_mode(mode)
        .pane_placement(placement)
        .open_pane_length(if wide { 240.0 } else { 160.0 })
        .compact_pane_length(48.0)
        .light_dismiss_overlay_mode(if dim {
            LightDismissOverlayMode::On
        } else {
            LightDismissOverlayMode::Off
        })
        .pane(pane)
        .content(content)
        .pane_opening(Listener::new(move |app| {
            self.set_state(app, |s| s.event = "PaneOpening")
        }))
        .pane_opened(Listener::new(move |app| {
            self.set_state(app, |s| s.event = "PaneOpened")
        }))
        .pane_closing(move |app, args| {
            args.cancel = app.get(self).cancel;
            self.set_state(app, |s| {
                s.event = if s.cancel {
                    "PaneClosing (cancel requested)"
                } else {
                    "PaneClosing"
                }
            });
        })
        .pane_closed(Listener::new(move |app| {
            self.set_state(app, |s| s.event = "PaneClosed")
        }));
        column(
            vec![
                controls,
                options,
                SizedBox::new()
                    .width(620.0)
                    .height(200.0)
                    .child(split)
                    .into_widget(),
                label(
                    format!("{mode_name} · {event}"),
                    TextBlockStyle::Caption,
                    resources.text_fill_color_secondary,
                ),
            ],
            8.0,
        )
    }
}

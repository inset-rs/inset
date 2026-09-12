//! `CupertinoButton`: the three constructors, the sizes, and the press fade.
//!
//! Press and hold any of the enabled ones — the opacity fade is 120 ms out and 180 ms back in,
//! which is why a quick tap looks instant and a held press does not. The disabled one never
//! fades: it takes no tap-down at all.

use inset_cupertino::{
    CupertinoButton, CupertinoButtonSize, CupertinoColors, CupertinoIcons, CupertinoTheme,
};
use inset_embedder::Size;
use inset_foundation::{App, Handle, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize};
use inset_widgets::{
    BuildContext, Icon, IntoWidget, Padding, Row, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::support::{
    body_style, screen, scrolling_body, secondary, section, section_with_footer, showcase,
};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    ButtonsDemo.into_widget()
}

/// Tap counts live here, in the smallest widget that renders them, rather than on a
/// gallery-wide model.
#[derive(Debug)]
pub struct ButtonsDemo;

/// Dart would spell this `_ButtonsDemoState`.
pub struct Counts {
    state: StateData<ButtonsDemo>,
    taps: u32,
    long_presses: u32,
}

impl StatefulWidget for ButtonsDemo {
    type State = Counts;

    fn create_state(&self) -> Counts {
        Counts {
            state: StateData::new(),
            taps: 0,
            long_presses: 0,
        }
    }
}

impl State for Counts {
    type Widget = ButtonsDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let count = Listener::new(move |app: &mut App| {
            self.set_state(app, |state| state.taps += 1);
        });
        let count_long = Listener::new(move |app: &mut App| {
            self.set_state(app, |state| state.long_presses += 1);
        });
        let theme = CupertinoTheme::of(app, context);
        let (taps, long_presses) = {
            let state = app.get(self);
            (state.taps, state.long_presses)
        };

        screen(scrolling_body(vec![
            section_with_footer(
                "Variants",
                "A button with no on_pressed is disabled, and does not fade when pressed.",
                vec![
                    showcase(CupertinoButton::new(
                        Text::new("Plain").into_widget(),
                        Some(count.clone()),
                    )),
                    showcase(CupertinoButton::tinted(
                        Text::new("Tinted").into_widget(),
                        Some(count.clone()),
                    )),
                    showcase(CupertinoButton::filled(
                        Text::new("Filled").into_widget(),
                        Some(count.clone()),
                    )),
                    showcase(
                        CupertinoButton::filled(
                            Text::new("Custom colour").into_widget(),
                            Some(count.clone()),
                        )
                        .color(CupertinoColors::SYSTEM_GREEN),
                    ),
                    showcase(CupertinoButton::filled(
                        Text::new("Disabled").into_widget(),
                        None,
                    )),
                ],
            ),
            section_with_footer(
                "Sizes",
                "Padding, minimum side and corner radius all come from the size.",
                vec![
                    showcase(
                        CupertinoButton::filled(
                            Text::new("Small").into_widget(),
                            Some(count.clone()),
                        )
                        .size_style(CupertinoButtonSize::Small),
                    ),
                    showcase(
                        CupertinoButton::filled(
                            Text::new("Medium").into_widget(),
                            Some(count.clone()),
                        )
                        .size_style(CupertinoButtonSize::Medium),
                    ),
                    showcase(
                        CupertinoButton::filled(
                            Text::new("Large").into_widget(),
                            Some(count.clone()),
                        )
                        .size_style(CupertinoButtonSize::Large),
                    ),
                ],
            ),
            section_with_footer(
                "Any child, and long press",
                "CupertinoButton::new takes a widget, not a label.",
                vec![
                    showcase(CupertinoButton::new(
                        Row::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Center)
                            .children([
                                Icon::new(Some(CupertinoIcons::share()))
                                    .size(18.0)
                                    .color(theme.primary_color())
                                    .into_widget(),
                                Padding::new(EdgeInsetsGeometry::from_ltrb(6.0, 0.0, 0.0, 0.0))
                                    .child(
                                        Text::new("Share")
                                            .style(theme.text_theme().action_text_style()),
                                    )
                                    .into_widget(),
                            ])
                            .into_widget(),
                        Some(count.clone()),
                    )),
                    showcase(
                        CupertinoButton::filled(Text::new("Hold me").into_widget(), Some(count))
                            .on_long_press(count_long)
                            .minimum_size(Size::new(180.0, 44.0)),
                    ),
                ],
            ),
            section(
                "Result",
                vec![
                    tally(app, context, "Taps", taps),
                    tally(app, context, "Long presses", long_presses),
                ],
            ),
        ]))
    }
}

fn tally(app: &mut App, context: BuildContext, label: &str, count: u32) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(12.0, 16.0))
        .child(
            Row::new()
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .children([
                    Text::new(label)
                        .style(body_style(app, context))
                        .into_widget(),
                    secondary(app, context, count.to_string()),
                ]),
        )
        .into_widget()
}

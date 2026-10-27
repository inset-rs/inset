//! Text styles, foreground hierarchy and a configurable reading sample.

use crate::{column, example, example_row, label, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// The complete type ramp currently exposed by the kit.
const STYLES: [(&str, TextBlockStyle); 9] = [
    ("Caption", TextBlockStyle::Caption),
    ("Body", TextBlockStyle::Body),
    ("Body Strong", TextBlockStyle::BodyStrong),
    ("Body Large", TextBlockStyle::BodyLarge),
    ("Body Large Strong", TextBlockStyle::BodyLargeStrong),
    ("Subtitle", TextBlockStyle::Subtitle),
    ("Title", TextBlockStyle::Title),
    ("Title Large", TextBlockStyle::TitleLarge),
    ("Display", TextBlockStyle::Display),
];

/// Builds the typography feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("Typography", resources, TypographyDemo.into_widget())
}

/// A page whose sample changes independently of the type ramp.
#[derive(Debug)]
struct TypographyDemo;

/// Retains sample choices while visiting other gallery pages.
struct TypographyDemoState {
    /// Native widget linkage.
    state: StateData<TypographyDemo>,
    /// Selected position in the supported type ramp.
    style: usize,
    /// Whether the sample uses secondary text emphasis.
    secondary: bool,
}

impl StatefulWidget for TypographyDemo {
    type State = TypographyDemoState;

    fn create_state(&self) -> Self::State {
        TypographyDemoState {
            state: StateData::new(),
            style: 1,
            secondary: false,
        }
    }
}

impl State for TypographyDemoState {
    type Widget = TypographyDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let (index, secondary) = (app.get(self).style, app.get(self).secondary);
        let (name, style) = STYLES[index];
        let sample = column(
            vec![
                example_row(
                    vec![
                        Button::text(
                            format!("Text style: {name}"),
                            Listener::new(move |app| {
                                self.set_state(app, |s| s.style = (s.style + 1) % STYLES.len())
                            }),
                        )
                        .into_widget(),
                        CheckBox::new(Some(secondary), move |app, value| {
                            self.set_state(app, |s| s.secondary = value.unwrap_or(false))
                        })
                        .content(Text::new("Secondary foreground"))
                        .into_widget(),
                    ],
                    16.0,
                ),
                label(
                    "Make the important things easy to read.",
                    style,
                    if secondary {
                        r.text_fill_color_secondary
                    } else {
                        r.text_fill_color_primary
                    },
                ),
            ],
            20.0,
        );
        let ramp = column(
            STYLES
                .into_iter()
                .map(|(name, style)| {
                    column(
                        vec![
                            label(name, style, r.text_fill_color_primary),
                            label(
                                format!("{} px", style.font(&r,).0),
                                TextBlockStyle::Caption,
                                r.text_fill_color_secondary,
                            ),
                        ],
                        4.0,
                    )
                })
                .collect(),
            24.0,
        );
        let emphasis = column(
            vec![
                label(
                    "Primary text carries the main message.",
                    TextBlockStyle::Body,
                    r.text_fill_color_primary,
                ),
                label(
                    "Secondary text adds supporting detail.",
                    TextBlockStyle::Body,
                    r.text_fill_color_secondary,
                ),
                label(
                    "Disabled text describes an unavailable option.",
                    TextBlockStyle::Body,
                    r.text_fill_color_disabled,
                ),
            ],
            12.0,
        );
        column(
            vec![
                example(
                    "Try a text style",
                    "Change the style and emphasis of the same sample.",
                    &r,
                    sample,
                ),
                example(
                    "Type ramp",
                    "Use smaller styles for supporting detail and larger styles for page headings.",
                    &r,
                    ramp,
                ),
                example(
                    "Foreground hierarchy",
                    "Text emphasis follows the gallery’s current theme.",
                    &r,
                    emphasis,
                ),
            ],
            24.0,
        )
    }
}

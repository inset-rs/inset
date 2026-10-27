//! InfoBadge display kinds, severity styles and the square minimum the source measures to.

use crate::{column, example, label, row, section};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::{CrossAxisAlignment, MainAxisSize};
use inset_widgets::*;
use inset_winui::*;

/// The named styles in the order the source dictionary declares them.
const STYLES: [(InfoBadgeStyle, &str); 6] = [
    (InfoBadgeStyle::Default, "Default"),
    (InfoBadgeStyle::Attention, "Attention"),
    (InfoBadgeStyle::Informational, "Informational"),
    (InfoBadgeStyle::Success, "Success"),
    (InfoBadgeStyle::Caution, "Caution"),
    (InfoBadgeStyle::Critical, "Critical"),
];

/// Counts that show the badge growing from a circle to a stadium.
const COUNTS: [i32; 4] = [0, 5, 42, 999];

/// Builds the badge feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("InfoBadge", resources, BadgeDemo.into_widget())
}

/// Interactive badge properties.
#[derive(Debug)]
struct BadgeDemo;

/// Retains the demonstrated count and style.
struct BadgeDemoState {
    /// Native widget linkage.
    state: StateData<BadgeDemo>,
    /// Position in the sample count list.
    count: usize,
    /// Position in the style list.
    style: usize,
}

impl StatefulWidget for BadgeDemo {
    type State = BadgeDemoState;

    fn create_state(&self) -> Self::State {
        BadgeDemoState {
            state: StateData::new(),
            count: 1,
            style: 0,
        }
    }
}

/// A badge above its caption.
fn captioned(badge: WidgetRef, name: &str, resources: &ThemeResources) -> WidgetRef {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .spacing(8.0)
        .children([
            SizedBox::new()
                .height(20.0)
                .child(Center::new().child(badge))
                .into_widget(),
            label(
                name,
                TextBlockStyle::Caption,
                resources.text_fill_color_secondary,
            ),
        ])
        .into_widget()
}

impl State for BadgeDemoState {
    type Widget = BadgeDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let (count, style) = {
            let state = app.get(self);
            (COUNTS[state.count], STYLES[state.style].0)
        };
        let kinds = row(
            vec![
                captioned(InfoBadge::new().style(style).into_widget(), "Dot", &r),
                captioned(
                    InfoBadge::new().style(style).value(count).into_widget(),
                    "Value",
                    &r,
                ),
                captioned(
                    InfoBadge::new()
                        .style(style)
                        .icon_source(FontIcon::symbol(FluentSymbol::Checkmark))
                        .into_widget(),
                    "FontIcon",
                    &r,
                ),
            ],
            28.0,
        );
        let options = row(
            vec![
                Button::text(
                    format!("Value: {count}"),
                    Listener::new(move |app| {
                        self.set_state(app, |s| s.count = (s.count + 1) % COUNTS.len())
                    }),
                )
                .into_widget(),
                Button::text(
                    format!("Style: {}", STYLES[app.get(self).style].1),
                    Listener::new(move |app| {
                        self.set_state(app, |s| s.style = (s.style + 1) % STYLES.len())
                    }),
                )
                .into_widget(),
            ],
            16.0,
        );
        let severities = row(
            STYLES
                .into_iter()
                .map(|(style, name)| {
                    captioned(
                        InfoBadge::new().style(style).value(9).into_widget(),
                        name,
                        &r,
                    )
                })
                .collect(),
            20.0,
        );
        let widths = row(
            COUNTS
                .into_iter()
                .map(|value| {
                    captioned(
                        InfoBadge::new().value(value).into_widget(),
                        &value.to_string(),
                        &r,
                    )
                })
                .collect(),
            20.0,
        );
        column(
            vec![
                example(
                    "Display kinds",
                    "A badge shows a value when one is set, an icon when one is supplied, and \
                     otherwise a bare dot.",
                    &r,
                    column(vec![options, kinds], 20.0),
                ),
                example(
                    "Severity",
                    "Each named style carries the fill its severity calls for.",
                    &r,
                    severities,
                ),
                example(
                    "Value width",
                    "A badge is never taller than it is wide, and a longer count widens it \
                     without making it taller.",
                    &r,
                    widths,
                ),
            ],
            24.0,
        )
    }
}

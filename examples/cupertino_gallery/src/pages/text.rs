//! Text: the theme's styles, what happens when a line does not fit, and what a larger
//! accessibility text size does to a layout.

use reveal_cupertino::{CupertinoColors, CupertinoDynamicColor, CupertinoTheme};
use reveal_embedder::FontWeight;
use reveal_foundation::App;
use reveal_painting::{EdgeInsetsGeometry, TextOverflow, TextScaler, TextSpan, TextStyle};
use reveal_rendering::CrossAxisAlignment;
use reveal_widgets::{
    BuildContext, Column, IntoWidget, MediaQuery, Padding, RichText, SizedBox, Text, WidgetRef,
};

use crate::support::{
    body_style, screen, scrolling_body, secondary, section_with_footer, showcase_leading,
};

/// Narrow enough that the sentence below cannot fit on one line, which is the only way an
/// overflow rule is visible.
const CLAMP_WIDTH: f64 = 200.0;

const LONG: &str = "A sentence long enough to need more room than it is given";

/// The accessibility scale to compare against 1.0.
const LARGE_SCALE: f64 = 1.6;

pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    let text_theme = CupertinoTheme::of(app, context).text_theme();

    screen(scrolling_body(vec![
        section_with_footer(
            "Theme styles",
            "Nothing on this page names a font or a point size.",
            vec![
                sample(app, context, "text_style", text_theme.text_style()),
                sample(
                    app,
                    context,
                    "action_text_style",
                    text_theme.action_text_style(),
                ),
                sample(
                    app,
                    context,
                    "action_small_text_style",
                    text_theme.action_small_text_style(),
                ),
                sample(
                    app,
                    context,
                    "nav_title_text_style",
                    text_theme.nav_title_text_style(),
                ),
                sample(
                    app,
                    context,
                    "nav_large_title_text_style",
                    text_theme.nav_large_title_text_style(),
                ),
            ],
        ),
        section_with_footer(
            "Overflow",
            "One sentence, one line, three rules — in the same 200-point box.",
            vec![
                overflow(app, context, "Clip", TextOverflow::Clip),
                overflow(app, context, "Ellipsis", TextOverflow::Ellipsis),
                // Fade currently behaves as Clip: `RenderParagraph` clips instead, because the
                // fade shader waits on gradients in the paint backend. Left in so the gap is
                // visible rather than quietly absent.
                overflow(app, context, "Fade (clips for now)", TextOverflow::Fade),
            ],
        ),
        section_with_footer(
            "Rich text",
            "One paragraph, three spans: a child span inherits the root's style and overrides \
             what it names.",
            vec![rich(app, context)],
        ),
        section_with_footer(
            "Text scaling",
            "The second block asked for nothing: its ancestor MediaQuery scales it.",
            vec![
                scaled(app, context, "1.0", None),
                scaled(app, context, "1.6", Some(LARGE_SCALE)),
            ],
        ),
    ]))
}

fn sample(app: &mut App, context: BuildContext, name: &str, style: TextStyle) -> WidgetRef {
    showcase_leading(
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children([
                secondary(app, context, name),
                Padding::new(EdgeInsetsGeometry::from_ltrb(0.0, 4.0, 0.0, 0.0))
                    .child(Text::new("The quick brown fox").style(style))
                    .into_widget(),
            ]),
    )
}

fn overflow(app: &mut App, context: BuildContext, name: &str, overflow: TextOverflow) -> WidgetRef {
    let style = body_style(app, context);
    showcase_leading(
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children([
                secondary(app, context, name),
                SizedBox::new()
                    .width(CLAMP_WIDTH)
                    .child(Text::new(LONG).style(style).max_lines(1).overflow(overflow))
                    .into_widget(),
            ]),
    )
}

/// A single paragraph built from spans rather than from one string.
///
/// The root span carries the body style and the children only name what they change, which is
/// how a `TextSpan` tree differs from three `Text`s in a `Row`: it wraps and breaks as one
/// paragraph.
fn rich(app: &mut App, context: BuildContext) -> WidgetRef {
    let style = body_style(app, context);
    let tint = CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_BLUE, app, context);
    showcase_leading(RichText::new(
        TextSpan::new()
            .style(style)
            .text("A paragraph with ")
            .children(vec![
                TextSpan::new()
                    .text("bold")
                    .style(TextStyle::new().font_weight(FontWeight::BOLD))
                    .into_span(),
                TextSpan::new().text(", ").into_span(),
                TextSpan::new()
                    .text("tinted")
                    .style(TextStyle::new().color(tint))
                    .into_span(),
                TextSpan::new().text(" and plain runs in it.").into_span(),
            ])
            .into_span(),
    ))
}

fn scaled(app: &mut App, context: BuildContext, name: &str, scale: Option<f64>) -> WidgetRef {
    let style = body_style(app, context);
    let secondary_color =
        CupertinoDynamicColor::resolve(&CupertinoColors::SECONDARY_LABEL, app, context);
    let block: WidgetRef = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .children([
            Text::new("Body text").style(style.clone()).into_widget(),
            Text::new("Secondary")
                .style(style.color(secondary_color))
                .into_widget(),
        ])
        .into_widget();

    let block = match scale {
        None => block,
        Some(scale) => MediaQuery::new(
            MediaQuery::of(app, context).text_scaler(TextScaler::linear(scale)),
            block,
        )
        .into_widget(),
    };

    showcase_leading(
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children([
                secondary(app, context, format!("text_scaler {name}")),
                Padding::new(EdgeInsetsGeometry::from_ltrb(0.0, 4.0, 0.0, 0.0))
                    .child(block)
                    .into_widget(),
            ]),
    )
}

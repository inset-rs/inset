//! `EmptyComingSoon.kt`: what the destinations other than the inbox show.

use waft_embedder::TextAlign;
use waft_foundation::App;
use waft_material::Theme;
use waft_painting::EdgeInsetsGeometry;
use waft_rendering::MainAxisAlignment;
use waft_widgets::{BuildContext, Column, IntoWidget, Padding, Text, WidgetRef};

pub fn empty_coming_soon(app: &mut App, context: BuildContext) -> WidgetRef {
    let theme = Theme::of(app, context);
    let title = theme
        .text_theme
        .title_large
        .clone()
        .unwrap_or_default()
        .color(theme.color_scheme.primary());
    let subtitle = theme
        .text_theme
        .body_small
        .clone()
        .unwrap_or_default()
        .color(theme.color_scheme.outline());
    Column::new()
        .main_axis_alignment(MainAxisAlignment::Center)
        .children([
            Padding::new(EdgeInsetsGeometry::all(8.0))
                .child(
                    Text::new("Screen under construction")
                        .style(title)
                        .text_align(TextAlign::Center),
                )
                .into_widget(),
            Padding::new(EdgeInsetsGeometry::symmetric(0.0, 8.0))
                .child(
                    Text::new(
                        "This screen is still under construction. This sample will help you learn about adaptive layouts in Jetpack Compose",
                    )
                    .style(subtitle)
                    .text_align(TextAlign::Center),
                )
                .into_widget(),
        ])
        .into_widget()
}

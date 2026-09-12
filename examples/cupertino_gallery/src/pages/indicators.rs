//! `CupertinoActivityIndicator` and `CupertinoLinearActivityIndicator`.
//!
//! The spinner's ticks are drawn, not animated by opacity on a stack of images: `animating:
//! false` freezes it, and the partially revealed constructor shows only the first fraction of
//! its ticks — which is what iOS does while a pull-to-refresh is still being dragged.

use inset_cupertino::{
    CupertinoActivityIndicator, CupertinoColors, CupertinoLinearActivityIndicator,
};
use inset_foundation::App;
use inset_rendering::{CrossAxisAlignment, MainAxisAlignment};
use inset_widgets::{BuildContext, Column, IntoWidget, Row, SizedBox, WidgetRef};

use crate::support::{
    screen, scrolling_body, secondary, section_with_footer, showcase, showcase_leading,
};

const BAR_WIDTH: f64 = 240.0;

pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    screen(scrolling_body(vec![
        section_with_footer(
            "Spinner",
            "Size is a radius, not a box: the ticks are drawn at that scale.",
            vec![showcase(
                Row::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
                    .children([
                        CupertinoActivityIndicator::new().into_widget(),
                        CupertinoActivityIndicator::new().radius(16.0).into_widget(),
                        CupertinoActivityIndicator::new()
                            .radius(24.0)
                            .color(CupertinoColors::SYSTEM_BLUE)
                            .into_widget(),
                    ]),
            )],
        ),
        section_with_footer(
            "Partially revealed",
            "Not animating — the state iOS draws while a refresh is still being pulled.",
            vec![showcase(
                Row::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
                    .children([0.25, 0.5, 0.75, 1.0].into_iter().map(|progress| {
                        CupertinoActivityIndicator::partially_revealed()
                            .progress(progress)
                            .into_widget()
                    })),
            )],
        ),
        section_with_footer(
            "Linear",
            "The determinate bar — a known fraction, not an unknown wait.",
            [0.15, 0.5, 0.85]
                .into_iter()
                .map(|progress| {
                    showcase_leading(
                        Column::new()
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .children([
                                secondary(app, context, format!("{:.0}%", progress * 100.0)),
                                SizedBox::new()
                                    .width(BAR_WIDTH)
                                    .child(CupertinoLinearActivityIndicator::new(progress))
                                    .into_widget(),
                            ]),
                    )
                })
                .collect(),
        ),
    ]))
}

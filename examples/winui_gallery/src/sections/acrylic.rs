//! Compares acrylic recipes with their opaque fallback over a quiet landscape.

use crate::{column, example, label, section};
use inset_embedder::{Canvas, Color, FillRule, Paint, PathBuilder, Size, valo};
use inset_foundation::App;
use inset_painting::{BorderRadius, BorderRadiusGeometry};
use inset_rendering::CustomPainter;
use inset_widgets::*;
use inset_winui::*;

/// Builds two surfaces using the source flyout material recipe.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    let recipe = resources.flyout_presenter_background;
    let panels = [
        ("Acrylic", Brush::Acrylic(recipe)),
        ("Opaque fallback", Brush::Solid(recipe.fallback_color)),
    ]
    .into_iter()
    .map(|(title, background)| {
        SizedBox::new()
            .width(f64::INFINITY)
            .height(200.0)
            .child(
                Stack::new().children([
                    Positioned::fill(
                        ClipRRect::new()
                            .border_radius(BorderRadiusGeometry::BorderRadius(
                                BorderRadius::circular(12.0),
                            ))
                            .child(CustomPaint::new().painter(Landscape)),
                    )
                    .into_widget(),
                    Positioned::fill(
                        Padding::new(inset_painting::EdgeInsetsGeometry::all(24.0)).child(
                            ControlBorder::new(background, Brush::Solid(Color::new(0)))
                                .border_thickness(0.0)
                                .corner_radius(8.0)
                                .padding([24.0, 24.0, 24.0, 24.0])
                                .child(column(
                                    vec![
                                        label(
                                            title,
                                            TextBlockStyle::Subtitle,
                                            resources.text_fill_color_primary,
                                        ),
                                        label(
                                            "Text and controls stay sharp.",
                                            TextBlockStyle::Body,
                                            resources.text_fill_color_primary,
                                        ),
                                    ],
                                    12.0,
                                )),
                        ),
                    )
                    .into_widget(),
                ]),
            )
            .into_widget()
    })
    .collect();
    section(
        "Acrylic",
        resources,
        example(
            "Recipes and fallback",
            "The same recipe can sample the scene through acrylic or use its opaque fallback color.",
            resources,
            column(panels, 24.0),
        ),
    )
}

/// Broad color fields reveal backdrop sampling without a distracting test pattern.
#[derive(Clone, Debug)]
struct Landscape;

impl CustomPainter for Landscape {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let width = size.width() as f32;
        let height = size.height() as f32;
        let gradient = |from, to| {
            Paint::from_shader(valo::Shader::linear(
                valo::Point::new(0.0, 0.0),
                valo::Point::new(width, height),
                Color::new(from).into(),
                Color::new(to).into(),
            ))
        };
        canvas.draw_rect(
            valo::Rect::new(0.0, 0.0, width, height),
            &gradient(0xFFE4DEE9, 0xFFB6D2DB),
        );
        for (start, first, second, end, from, to) in [
            (0.82, -0.20, 0.18, 0.46, 0xFFB2B3D5, 0xFF829DBE),
            (0.54, 1.10, -0.18, 0.72, 0xFF86AEBB, 0xFFCAD7D6),
            (0.92, 0.30, 1.20, 0.66, 0xFF587F9D, 0xFFA0BCCC),
        ] {
            let mut path = PathBuilder::new();
            path.move_to((0.0, height * start));
            path.cubic_to(
                (width * 0.32, height * first),
                (width * 0.68, height * second),
                (width, height * end),
            );
            path.line_to((width, height));
            path.line_to((0.0, height));
            path.close();
            canvas.draw_path(&path.build(), FillRule::NonZero, &gradient(from, to));
        }
    }

    fn should_repaint(&self, _app: &App, _old: &dyn CustomPainter) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

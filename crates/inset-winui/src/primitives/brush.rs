//! XAML brushes as the templates use them: solid colours, elevation gradients, and in-window acrylic.

use crate::AcrylicBrushResources;
use inset_embedder::valo::{
    GradientStop, Matrix, Paint, PaintStyle, Point, Shader, SpreadMode, Stroke,
};
use inset_embedder::{
    Canvas, ClipOp, Color, FillRule, PathBuilder, RRect, Radius, Rect, rrect_radii_elliptical,
};
use inset_painting::{draw_drrect, draw_oval, draw_rrect};

/// A XAML `Brush` a template paints with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brush {
    /// `SolidColorBrush`.
    Solid(Color),
    /// In-window AcrylicBrush, sampling the already painted scene beneath the shape.
    Acrylic(AcrylicBrushResources),
    /// `ControlElevationBorderBrush` and `AccentControlElevationBorderBrush`: an absolute vertical gradient over the bottom three pixels of the element (XAML `MappingMode="Absolute"`, `EndPoint="0,3"`, flipped by `ScaleY="-1"`), so the bottom edge of a border reads darker. Stops are (offset, colour) as recorded.
    ControlElevation([(f64, Color); 2]),
    /// `CircleElevationBorderBrush`: a vertical gradient over the whole element (`MappingMode="RelativeToBoundingBox"`), for round knobs.
    CircleElevation([(f64, Color); 2]),
    /// An absolute vertical gradient rising from the bottom edge, as in TabView's selected border.
    VerticalElevation {
        /// Source gradient stops.
        stops: [(f64, Color); 2],
        /// The absolute gradient length in logical pixels.
        length: f64,
    },
}

impl Brush {
    /// A representative colour: the solid colour, the last gradient stop, or acrylic’s fallback.
    /// This is an approximation for callers that need one colour, not material painting.
    pub fn representative_color(self) -> Color {
        match self {
            Brush::Solid(color) => color,
            Brush::Acrylic(resources) => resources.fallback_color,
            Brush::ControlElevation(stops) | Brush::CircleElevation(stops) => stops[1].1,
            Brush::VerticalElevation { stops, .. } => stops[1].1,
        }
    }

    /// Ordinary paint is absent for materials, which need a backdrop-sampling pass.
    fn fill(self, bounds: Rect) -> Option<Paint> {
        Some(match self {
            Brush::Acrylic(_) => return None,
            Brush::Solid(color) => Paint::from_color(color.into()),
            Brush::ControlElevation(stops) => Paint::from_shader(Shader::Linear {
                start: Point::new(bounds.left as f32, bounds.bottom as f32),
                end: Point::new(bounds.left as f32, bounds.bottom as f32 - 3.0),
                stops: gradient_stops(stops),
                spread: SpreadMode::Pad,
                local: Matrix::IDENTITY,
            }),
            Brush::CircleElevation(stops) => Paint::from_shader(Shader::Linear {
                start: Point::new(bounds.left as f32, bounds.top as f32),
                end: Point::new(bounds.left as f32, bounds.bottom as f32),
                stops: gradient_stops(stops),
                spread: SpreadMode::Pad,
                local: Matrix::IDENTITY,
            }),
            Brush::VerticalElevation { stops, length } => Paint::from_shader(Shader::Linear {
                start: Point::new(bounds.left as f32, bounds.bottom as f32),
                end: Point::new(bounds.left as f32, bounds.bottom as f32 - length as f32),
                stops: gradient_stops(stops),
                spread: SpreadMode::Pad,
                local: Matrix::IDENTITY,
            }),
        })
    }

    /// Paints a rounded rectangle, using `bounds` as the brush's gradient/material origin.
    pub fn paint_rrect(self, canvas: &mut Canvas, rect: RRect, bounds: Rect) {
        if let Some(paint) = self.fill(bounds) {
            draw_rrect(canvas, rect, &paint);
        } else {
            canvas.save();
            canvas.clip_rrect_radii_elliptical(
                rect.outer_rect(),
                rrect_radii_elliptical(rect),
                ClipOp::Intersect,
            );
            self.paint_material(canvas, rect.outer_rect());
            canvas.restore();
        }
    }

    /// Paints the ring between two rounded rectangles without filtering their interior.
    pub fn paint_drrect(self, canvas: &mut Canvas, outer: RRect, inner: RRect, bounds: Rect) {
        if let Some(paint) = self.fill(bounds) {
            draw_drrect(canvas, outer, inner, &paint);
        } else {
            let mut path = PathBuilder::new();
            path.rrect_radii_elliptical(outer.outer_rect(), rrect_radii_elliptical(outer));
            path.rrect_radii_elliptical(inner.outer_rect(), rrect_radii_elliptical(inner));
            canvas.save();
            canvas.clip_path(&path.build(), FillRule::EvenOdd, ClipOp::Intersect);
            self.paint_material(canvas, outer.outer_rect());
            canvas.restore();
        }
    }

    /// Strokes the rounded outline with a centred stroke of `width` logical pixels.
    pub fn paint_rrect_stroke(self, canvas: &mut Canvas, rect: RRect, bounds: Rect, width: f64) {
        if let Some(mut paint) = self.fill(bounds) {
            paint.style = PaintStyle::Stroke(Stroke::new(width as f32));
            draw_rrect(canvas, rect, &paint);
        } else {
            self.paint_drrect(
                canvas,
                rect.inflate(width / 2.0),
                rect.deflate(width / 2.0),
                bounds,
            );
        }
    }

    /// Paints an oval inscribed in `rect`.
    pub fn paint_oval(self, canvas: &mut Canvas, rect: Rect, bounds: Rect) {
        if let Some(paint) = self.fill(bounds) {
            draw_oval(canvas, rect, &paint);
        } else {
            self.paint_rrect(
                canvas,
                RRect::from_rect_and_radius(
                    rect,
                    Radius::elliptical(rect.width() / 2.0, rect.height() / 2.0),
                ),
                bounds,
            );
        }
    }

    /// Strokes an oval with a centred stroke of `width` logical pixels.
    pub fn paint_oval_stroke(self, canvas: &mut Canvas, rect: Rect, bounds: Rect, width: f64) {
        if let Some(mut paint) = self.fill(bounds) {
            paint.style = PaintStyle::Stroke(Stroke::new(width as f32));
            draw_oval(canvas, rect, &paint);
        } else {
            self.paint_rrect_stroke(
                canvas,
                RRect::from_rect_and_radius(
                    rect,
                    Radius::elliptical(rect.width() / 2.0, rect.height() / 2.0),
                ),
                bounds,
                width,
            );
        }
    }

    /// A material pass closes before the caller draws any foreground children.
    fn paint_material(self, canvas: &mut Canvas, bounds: Rect) {
        if let Brush::Acrylic(resources) = self {
            super::acrylic::paint_acrylic(canvas, bounds, resources);
        }
    }
}

fn gradient_stops(stops: [(f64, Color); 2]) -> Vec<GradientStop> {
    stops
        .into_iter()
        .map(|(offset, color)| GradientStop {
            offset: offset as f32,
            color: color.into(),
        })
        .collect()
}

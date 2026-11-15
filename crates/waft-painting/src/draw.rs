//! Bridges from painting geometry to valo draw calls.
//!
//! valo draws rounded rects natively but has no `drawDRRect` primitive; the
//! double-rounded-rect (a border ring) is one even-odd path holding the outer
//! and inner contours.

use waft_embedder::valo::FillRule;
use waft_embedder::{Canvas, Offset, Paint, PaintStyle, PathBuilder, Stroke};
use waft_embedder::{
    RRect, RSuperellipse, Rect, rrect_radii_elliptical, rsuperellipse_radii_elliptical,
};

/// Draws a line between `p1` and `p2` with `paint` (Flutter `Canvas.drawLine`).
///
/// The line is always stroked, as Dart's `drawLine` ignores `Paint.style`. A valo paint keeps its
/// stroke width in its stroke style, so a fill paint strokes at Dart's default `strokeWidth`, 0,
/// a hairline.
pub fn draw_line(canvas: &mut Canvas, p1: Offset, p2: Offset, paint: &Paint) {
    let mut path = PathBuilder::new();
    path.move_to(p1);
    path.line_to(p2);
    if let PaintStyle::Stroke(_) = paint.style {
        canvas.draw_path(&path.build(), FillRule::NonZero, paint);
        return;
    }
    let mut stroked = paint.clone();
    stroked.style = PaintStyle::Stroke(Stroke::new(0.0));
    canvas.draw_path(&path.build(), FillRule::NonZero, &stroked);
}

/// Draws a rounded rect, falling back to the plain rect op when every corner
/// is sharp (Flutter's `drawRect` vs `drawRRect` split).
pub fn draw_rrect(canvas: &mut Canvas, rrect: RRect, paint: &Paint) {
    if rrect.tl_radius_x == 0.0
        && rrect.tl_radius_y == 0.0
        && rrect.tr_radius_x == 0.0
        && rrect.tr_radius_y == 0.0
        && rrect.br_radius_x == 0.0
        && rrect.br_radius_y == 0.0
        && rrect.bl_radius_x == 0.0
        && rrect.bl_radius_y == 0.0
    {
        canvas.draw_rect(rrect.outer_rect(), paint);
    } else {
        canvas.draw_rrect_radii_elliptical(
            rrect.outer_rect(),
            rrect_radii_elliptical(rrect),
            paint,
        );
    }
}

/// Draws the ring between `outer` and `inner` (Flutter `Canvas.drawDRRect`).
pub fn draw_drrect(canvas: &mut Canvas, outer: RRect, inner: RRect, paint: &Paint) {
    let mut path = PathBuilder::new();
    path.rrect_radii_elliptical(outer.outer_rect(), rrect_radii_elliptical(outer));
    path.rrect_radii_elliptical(inner.outer_rect(), rrect_radii_elliptical(inner));
    canvas.draw_path(&path.build(), FillRule::EvenOdd, paint);
}

/// Draws an oval inscribed in `rect` (Flutter `Canvas.drawOval`).
pub fn draw_oval(canvas: &mut Canvas, rect: Rect, paint: &Paint) {
    let rx = (rect.width() / 2.0) as f32;
    let ry = (rect.height() / 2.0) as f32;
    canvas.draw_rrect_radii_elliptical(rect, [[rx, ry]; 4], paint);
}

/// Draws the arc of the oval inscribed in `rect`, from `start_angle` clockwise through
/// `sweep_angle` radians, closed back to the centre when `use_center` (Flutter
/// `Canvas.drawArc`). An empty `rect` or a zero sweep draws nothing, as Skia's `drawArc` does.
pub fn draw_arc(
    canvas: &mut Canvas,
    rect: Rect,
    start_angle: f64,
    sweep_angle: f64,
    use_center: bool,
    paint: &Paint,
) {
    if rect.is_empty() || sweep_angle == 0.0 {
        return;
    }
    let mut path = PathBuilder::new();
    if use_center {
        path.move_to(rect.center());
    }
    path.ellipse(
        rect.center(),
        [(rect.width() / 2.0) as f32, (rect.height() / 2.0) as f32],
        0.0,
        start_angle as f32,
        sweep_angle as f32,
    );
    if use_center {
        path.close();
    }
    canvas.draw_path(&path.build(), FillRule::NonZero, paint);
}

/// Draws a rounded superellipse (Flutter `Canvas.drawRSuperellipse`).
pub fn draw_rsuperellipse(canvas: &mut Canvas, rse: RSuperellipse, paint: &Paint) {
    if rse.tl_radius_x == 0.0
        && rse.tl_radius_y == 0.0
        && rse.tr_radius_x == 0.0
        && rse.tr_radius_y == 0.0
        && rse.br_radius_x == 0.0
        && rse.br_radius_y == 0.0
        && rse.bl_radius_x == 0.0
        && rse.bl_radius_y == 0.0
    {
        canvas.draw_rect(rse.outer_rect(), paint);
    } else {
        let mut path = PathBuilder::new();
        path.rsuperellipse_radii(rse.outer_rect().into(), rsuperellipse_radii_elliptical(rse));
        canvas.draw_path(&path.build(), FillRule::NonZero, paint);
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use waft_embedder::valo::{Op, Point};
    use waft_embedder::{Canvas, Color, Paint, Rect};

    use super::draw_arc;

    fn arc_ops(rect: Rect, start_angle: f64, sweep_angle: f64, use_center: bool) -> Vec<Op> {
        let mut canvas = Canvas::new();
        let paint = Paint::from_color(Color::new(0xFF000000).into());
        draw_arc(
            &mut canvas,
            rect,
            start_angle,
            sweep_angle,
            use_center,
            &paint,
        );
        canvas.build().ops().to_vec()
    }

    #[test]
    fn draw_arc_draws_nothing_for_an_empty_rect_or_a_zero_sweep() {
        let square = Rect::from_ltwh(0.0, 0.0, 10.0, 10.0);
        assert!(arc_ops(Rect::from_ltwh(0.0, 0.0, 0.0, 10.0), 0.0, PI, false).is_empty());
        assert!(arc_ops(square, PI / 2.0, 0.0, false).is_empty());
        assert_eq!(arc_ops(square, 0.0, PI, false).len(), 1);
    }

    #[test]
    fn draw_arc_closes_back_to_the_center_only_when_asked() {
        let square = Rect::from_ltwh(0.0, 0.0, 10.0, 10.0);
        let covers_center = |use_center| {
            let ops = arc_ops(square, PI / 4.0, PI / 2.0, use_center);
            let [Op::DrawPath { path, .. }] = ops.as_slice() else {
                panic!("one path: {ops:?}");
            };
            path.bounds().contains(Point::new(5.0, 6.0))
        };
        assert!(covers_center(true));
        assert!(!covers_center(false));
    }
}

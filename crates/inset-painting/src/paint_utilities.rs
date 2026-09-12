//! Flutter counterpart: `painting/paint_utilities.dart`.

use inset_embedder::{Canvas, FillRule, Offset, Paint, PathBuilder};

/// Draw a line between two points, which cuts diagonally back and forth across
/// the line that connects the two points.
///
/// The line will cross the line `zigs - 1` times.
///
/// If `zigs` is 1, then this will draw two sides of a triangle from `start` to
/// `end`, with the third point being `width` away from the line, as measured
/// perpendicular to that line.
///
/// If `width` is positive, the first `zig` will be to the left of the `start`
/// point when facing the `end` point. To reverse the zigging polarity, provide
/// a negative `width`.
///
/// The line is drawn using the provided `paint` on the provided `canvas`.
pub fn paint_zig_zag(
    canvas: &mut Canvas,
    paint: &Paint,
    start: Offset,
    end: Offset,
    zigs: i32,
    width: f64,
) {
    debug_assert!(zigs > 0);
    canvas.save();
    canvas.translate(start.dx() as f32, start.dy() as f32);
    let end = end - start;
    canvas.rotate(end.dy().atan2(end.dx()) as f32);
    let length = end.distance();
    let spacing = length / (zigs as f64 * 2.0);
    let mut path = PathBuilder::new();
    path.move_to(Offset::ZERO);
    for index in 0..zigs {
        let x = (f64::from(index) * 2.0 + 1.0) * spacing;
        let y = width * ((f64::from(index) % 2.0) * 2.0 - 1.0);
        path.line_to(Offset::new(x, y));
    }
    path.line_to(Offset::new(length, 0.0));
    canvas.draw_path(&path.build(), FillRule::NonZero, paint);
    canvas.restore();
}

//! Bridges from painting geometry to valo draw calls.
//!
//! valo draws rounded rects natively but has no `drawDRRect` primitive; the
//! double-rounded-rect (a border ring) is one even-odd path holding the outer
//! and inner contours.

use reveal_embedder::valo::FillRule;
use reveal_embedder::{Canvas, Paint, PathBuilder};
use reveal_embedder::{RRect, rrect_radii_elliptical};

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

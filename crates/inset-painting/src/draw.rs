//! Bridges from painting geometry to valo draw calls.
//!
//! valo draws rounded rects natively but has no `drawDRRect` primitive; the
//! double-rounded-rect (a border ring) is one even-odd path holding the outer
//! and inner contours.

use inset_embedder::valo::FillRule;
use inset_embedder::{Canvas, Paint, PathBuilder};
use inset_embedder::{
    RRect, RSuperellipse, Rect, rrect_radii_elliptical, rsuperellipse_radii_elliptical,
};

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

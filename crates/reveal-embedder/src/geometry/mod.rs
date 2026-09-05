//! Flutter counterpart: the pure-Dart value layer of `dart:ui`, which lives in
//! the engine at `engine/src/flutter/lib/ui`.

mod clip;
mod color;
#[allow(clippy::module_inception)]
mod geometry;
mod lerp;
mod math;
mod rrect;
mod shadow;

pub use clip::*;
pub use color::*;
pub use geometry::*;
pub use lerp::*;
pub use math::*;
pub use rrect::*;
pub use shadow::*;
pub use valo::Matrix as Matrix4;
pub use valo_geometry::MatrixKind;

mod matrix;
pub use matrix::*;

impl From<Color> for valo::Color {
    fn from(color: Color) -> valo::Color {
        valo::Color {
            r: color.r as f32,
            g: color.g as f32,
            b: color.b as f32,
            a: color.a as f32,
        }
    }
}

impl From<valo::Color> for Color {
    fn from(color: valo::Color) -> Color {
        Color::from(
            color.a as f64,
            color.r as f64,
            color.g as f64,
            color.b as f64,
            ColorSpace::Srgb,
        )
    }
}

impl From<Rect> for valo::Rect {
    fn from(rect: Rect) -> valo::Rect {
        valo::Rect::from_ltrb(
            rect.left as f32,
            rect.top as f32,
            rect.right as f32,
            rect.bottom as f32,
        )
    }
}

impl From<Offset> for valo::Point {
    fn from(offset: Offset) -> valo::Point {
        valo::Point {
            x: offset.dx() as f32,
            y: offset.dy() as f32,
        }
    }
}

impl From<valo::Point> for Offset {
    fn from(point: valo::Point) -> Offset {
        Offset::new(point.x as f64, point.y as f64)
    }
}

impl From<Size> for valo::Size {
    fn from(size: Size) -> valo::Size {
        valo::Size {
            width: size.width() as f32,
            height: size.height() as f32,
        }
    }
}

/// Per-corner elliptical radii in valo's order (clockwise from top-left).
pub fn rrect_radii_elliptical(rrect: RRect) -> [[f32; 2]; 4] {
    corner_radii_elliptical([
        [rrect.tl_radius_x, rrect.tl_radius_y],
        [rrect.tr_radius_x, rrect.tr_radius_y],
        [rrect.br_radius_x, rrect.br_radius_y],
        [rrect.bl_radius_x, rrect.bl_radius_y],
    ])
}

/// Per-corner elliptical radii in valo's order (clockwise from top-left).
pub fn rsuperellipse_radii_elliptical(rse: RSuperellipse) -> [[f32; 2]; 4] {
    corner_radii_elliptical([
        [rse.tl_radius_x, rse.tl_radius_y],
        [rse.tr_radius_x, rse.tr_radius_y],
        [rse.br_radius_x, rse.br_radius_y],
        [rse.bl_radius_x, rse.bl_radius_y],
    ])
}

fn corner_radii_elliptical(radii: [[f64; 2]; 4]) -> [[f32; 2]; 4] {
    radii.map(|[x, y]| [x as f32, y as f32])
}

#[cfg(test)]
mod valo_interop_tests {
    use super::*;

    #[test]
    fn color_channels_narrow_to_f32() {
        let valo_color = valo::Color::from(Color::from_argb(255, 255, 0, 128));
        assert_eq!(valo_color.a, 1.0);
        assert_eq!(valo_color.r, 1.0);
        assert_eq!(valo_color.g, 0.0);
        assert!((valo_color.b - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn rect_narrows_to_f32_sides() {
        let valo_rect = valo::Rect::from(Rect::from_ltrb(1.5, 2.5, 3.5, 4.5));
        assert_eq!(
            (valo_rect.x, valo_rect.y, valo_rect.width, valo_rect.height),
            (1.5, 2.5, 2.0, 2.0)
        );
    }
}

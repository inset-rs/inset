//! Flutter counterpart: the pure-Dart value layer of `dart:ui`, which lives in
//! the engine at `engine/src/flutter/lib/ui`.

mod clip;
mod color;
mod geometry;
mod lerp;
mod math;
mod rrect;

pub use clip::*;
pub use color::*;
pub use geometry::*;
pub use lerp::*;
pub use math::*;
pub use rrect::*;
pub use valo_geometry::{Matrix as Matrix4, MatrixKind};

mod matrix;
pub use matrix::transform3;

impl From<Color> for valo_geometry::Color {
    fn from(color: Color) -> valo_geometry::Color {
        valo_geometry::Color {
            r: color.r as f32,
            g: color.g as f32,
            b: color.b as f32,
            a: color.a as f32,
        }
    }
}

impl From<valo_geometry::Color> for Color {
    fn from(color: valo_geometry::Color) -> Color {
        Color::from(
            color.a as f64,
            color.r as f64,
            color.g as f64,
            color.b as f64,
            ColorSpace::Srgb,
        )
    }
}

impl From<Rect> for valo_geometry::Rect {
    fn from(rect: Rect) -> valo_geometry::Rect {
        valo_geometry::Rect::from_ltrb(
            rect.left as f32,
            rect.top as f32,
            rect.right as f32,
            rect.bottom as f32,
        )
    }
}

impl From<Offset> for valo_geometry::Point {
    fn from(offset: Offset) -> valo_geometry::Point {
        valo_geometry::Point {
            x: offset.dx() as f32,
            y: offset.dy() as f32,
        }
    }
}

impl From<valo_geometry::Point> for Offset {
    fn from(point: valo_geometry::Point) -> Offset {
        Offset::new(point.x as f64, point.y as f64)
    }
}

impl From<Size> for valo_geometry::Size {
    fn from(size: Size) -> valo_geometry::Size {
        valo_geometry::Size {
            width: size.width() as f32,
            height: size.height() as f32,
        }
    }
}

/// Per-corner elliptical radii in valo's order (clockwise from top-left).
pub fn rrect_radii_elliptical(rrect: RRect) -> [[f32; 2]; 4] {
    [
        [rrect.tl_radius_x as f32, rrect.tl_radius_y as f32],
        [rrect.tr_radius_x as f32, rrect.tr_radius_y as f32],
        [rrect.br_radius_x as f32, rrect.br_radius_y as f32],
        [rrect.bl_radius_x as f32, rrect.bl_radius_y as f32],
    ]
}

#[cfg(test)]
mod valo_interop_tests {
    use super::*;

    #[test]
    fn color_channels_narrow_to_f32() {
        let valo_color = valo_geometry::Color::from(Color::from_argb(255, 255, 0, 128));
        assert_eq!(valo_color.a, 1.0);
        assert_eq!(valo_color.r, 1.0);
        assert_eq!(valo_color.g, 0.0);
        assert!((valo_color.b - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn rect_narrows_to_f32_sides() {
        let valo_rect = valo_geometry::Rect::from(Rect::from_ltrb(1.5, 2.5, 3.5, 4.5));
        assert_eq!(
            (valo_rect.x, valo_rect.y, valo_rect.width, valo_rect.height),
            (1.5, 2.5, 2.0, 2.0)
        );
    }
}

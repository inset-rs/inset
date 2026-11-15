//! Flutter counterpart: `cupertino_ui/thumb_painter.dart`.

use waft_embedder::{Canvas, Color, Offset, Paint, RRect, Radius, Rect};
use waft_painting::{BlurStyle, BoxShadow, draw_rrect};

use crate::colors::CupertinoColors;

const K_THUMB_BORDER_COLOR: Color = Color::new(0x0A000000);

/// Dart's `_kSwitchBoxShadows`.
fn k_switch_box_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(
            Color::new(0x26000000),
            Offset::new(0.0, 3.0),
            8.0,
            0.0,
            BlurStyle::Normal,
        ),
        BoxShadow::new(
            Color::new(0x0F000000),
            Offset::new(0.0, 3.0),
            1.0,
            0.0,
            BlurStyle::Normal,
        ),
    ]
}

/// Dart's `_kSliderBoxShadows`.
fn k_slider_box_shadows() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(
            Color::new(0x26000000),
            Offset::new(0.0, 3.0),
            8.0,
            0.0,
            BlurStyle::Normal,
        ),
        BoxShadow::new(
            Color::new(0x29000000),
            Offset::new(0.0, 1.0),
            1.0,
            0.0,
            BlurStyle::Normal,
        ),
        BoxShadow::new(
            Color::new(0x1A000000),
            Offset::new(0.0, 3.0),
            1.0,
            0.0,
            BlurStyle::Normal,
        ),
    ]
}

/// Paints an iOS-style slider thumb or switch thumb.
///
/// Used by [`CupertinoSlider`](crate::CupertinoSlider).
#[derive(Clone, Debug)]
pub struct CupertinoThumbPainter {
    /// The color of the interior of the thumb.
    pub color: Color,
    /// The list of [`BoxShadow`] to paint below the thumb.
    pub shadows: Vec<BoxShadow>,
}

impl CupertinoThumbPainter {
    /// Half the default diameter of the thumb.
    pub const RADIUS: f64 = 14.0;

    /// The default amount the thumb should be extended horizontally when pressed.
    pub const EXTENSION: f64 = 7.0;

    /// Creates an object that paints an iOS-style slider thumb.
    pub fn new() -> CupertinoThumbPainter {
        CupertinoThumbPainter::default()
    }

    /// Creates an object that paints an iOS-style switch thumb.
    pub fn switch_thumb() -> CupertinoThumbPainter {
        CupertinoThumbPainter::new().shadows(k_switch_box_shadows())
    }

    /// Dart `CupertinoThumbPainter(color:)`.
    pub fn color(mut self, color: Color) -> CupertinoThumbPainter {
        self.color = color;
        self
    }

    /// Dart `CupertinoThumbPainter(shadows:)`.
    pub fn shadows(mut self, shadows: Vec<BoxShadow>) -> CupertinoThumbPainter {
        self.shadows = shadows;
        self
    }

    /// Paints the thumb onto the given canvas in the given rectangle.
    ///
    /// Consider using [`RADIUS`](Self::RADIUS) and [`EXTENSION`](Self::EXTENSION) when deciding
    /// how large a rectangle to use for the thumb.
    pub fn paint(&self, canvas: &mut Canvas, rect: Rect) {
        // Paint RRects instead of RSuperellipses here, because practically
        // [CupertinoSlider] only draws circular thumbs.
        let thumb_shape =
            RRect::from_rect_and_radius(rect, Radius::circular(rect.shortest_side() / 2.0));

        for shadow in &self.shadows {
            draw_rrect(canvas, thumb_shape.shift(shadow.offset), &shadow.to_paint());
        }

        draw_rrect(
            canvas,
            thumb_shape.inflate(0.5),
            &Paint::from_color(K_THUMB_BORDER_COLOR.into()),
        );
        draw_rrect(canvas, thumb_shape, &Paint::from_color(self.color.into()));
    }
}

impl Default for CupertinoThumbPainter {
    fn default() -> CupertinoThumbPainter {
        CupertinoThumbPainter {
            color: CupertinoColors::WHITE.color(),
            shadows: k_slider_box_shadows(),
        }
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::valo::Op;

    use super::*;

    fn colors(painter: &CupertinoThumbPainter) -> Vec<waft_embedder::valo::Color> {
        let mut canvas = Canvas::new();
        painter.paint(&mut canvas, Rect::from_ltwh(0.0, 0.0, 28.0, 28.0));
        canvas
            .build()
            .ops()
            .iter()
            .filter_map(|op| match op {
                Op::DrawPath { paint, .. }
                | Op::DrawRect { paint, .. }
                | Op::RRectBlur { paint, .. } => Some(paint.color),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_slider_thumb_paints_three_shadows_a_border_and_its_colour() {
        let painter = CupertinoThumbPainter::new().color(Color::new(0xFF112233));
        let colors = colors(&painter);
        let expected: Vec<waft_embedder::valo::Color> =
            [0x26000000, 0x29000000, 0x1A000000, 0x0A000000, 0xFF112233]
                .into_iter()
                .map(|argb| Color::new(argb).into())
                .collect();
        assert_eq!(colors, expected);
    }

    #[test]
    fn the_switch_thumb_paints_two_shadows_a_border_and_white() {
        let colors = colors(&CupertinoThumbPainter::switch_thumb());
        let expected: Vec<waft_embedder::valo::Color> =
            [0x26000000, 0x0F000000, 0x0A000000, 0xFFFFFFFF]
                .into_iter()
                .map(|argb| Color::new(argb).into())
                .collect();
        assert_eq!(colors, expected);
    }
}

//! Flutter counterpart: `painting/borders.dart` (`BorderStyle`, `BorderSide`,
//! `paintBorder`). `ShapeBorder` / `OutlinedBorder` are not here yet.

use std::fmt::{Debug, Formatter, Result as FmtResult};

use reveal_embedder::{Canvas, FillRule, Paint, PaintStyle, PathBuilder, Stroke};
use reveal_geometry::{Color, Offset, Rect, lerp_double};

/// The style of line to draw for a [`BorderSide`] in a `Border`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// Skip the border.
    None,
    /// Draw the border as a solid line.
    Solid,
}

/// A side of a border of a box.
///
/// A `Border` consists of four [`BorderSide`] objects.
///
/// Setting [`width`](BorderSide::width) to 0.0 will result in hairline rendering;
/// see [`width`](BorderSide::width) for a more involved explanation.
#[derive(Clone, Copy, PartialEq)]
pub struct BorderSide {
    /// The color of this side of the border.
    pub color: Color,
    /// The width of this side of the border, in logical pixels.
    ///
    /// Setting width to 0.0 will result in a hairline border. This means that
    /// the border will have the width of one physical pixel. Hairline
    /// rendering takes shortcuts when the path overlaps a pixel more than once.
    /// This means that it will render faster than otherwise, but it might
    /// double-hit pixels, giving it a slightly darker/lighter result.
    ///
    /// To omit the border entirely, set the [`style`](BorderSide::style) to
    /// [`BorderStyle::None`].
    pub width: f64,
    /// The style of this side of the border.
    ///
    /// To omit a side, set [`style`](BorderSide::style) to [`BorderStyle::None`].
    /// This skips painting the border, but the border still has a
    /// [`width`](BorderSide::width).
    pub style: BorderStyle,
    /// The relative position of the stroke on a [`BorderSide`] in an
    /// `OutlinedBorder` or `Border`.
    ///
    /// Values typically range from -1.0 ([`STROKE_ALIGN_INSIDE`], inside border,
    /// default) to 1.0 ([`STROKE_ALIGN_OUTSIDE`], outside border), without any
    /// bound constraints. A value of 0 ([`STROKE_ALIGN_CENTER`]) will center the
    /// border on the edge of the widget.
    ///
    /// This property is not honored by [`to_paint`](Self::to_paint) (because the
    /// [`Paint`] object cannot represent it); it is intended that classes that
    /// use [`BorderSide`] objects implement this property when painting borders
    /// by suitably inflating or deflating their regions.
    pub stroke_align: f64,
}

impl BorderSide {
    /// The border is drawn fully inside of the border path.
    pub const STROKE_ALIGN_INSIDE: f64 = -1.0;
    /// The border is drawn on the center of the border path.
    pub const STROKE_ALIGN_CENTER: f64 = 0.0;
    /// The border is drawn on the outside of the border path.
    pub const STROKE_ALIGN_OUTSIDE: f64 = 1.0;

    /// A hairline black border that is not rendered.
    pub const NONE: BorderSide = BorderSide {
        color: Color::new(0xFF000000),
        width: 0.0,
        style: BorderStyle::None,
        stroke_align: Self::STROKE_ALIGN_INSIDE,
    };

    /// Creates the side of a border.
    ///
    /// By default, the border is 1.0 logical pixels wide and solid black.
    pub fn new(color: Color, width: f64, style: BorderStyle, stroke_align: f64) -> BorderSide {
        debug_assert!(width >= 0.0);
        BorderSide {
            color,
            width,
            style,
            stroke_align,
        }
    }

    /// Creates a [`BorderSide`] that represents the addition of the two given
    /// [`BorderSide`]s.
    ///
    /// It is only valid to call this if [`can_merge`](Self::can_merge) returns
    /// true for the two sides.
    ///
    /// If one of the sides is zero-width with [`BorderStyle::None`], then the
    /// other side is returned as-is. If both of the sides are zero-width with
    /// [`BorderStyle::None`], then [`BorderSide::NONE`] is returned.
    pub fn merge(a: BorderSide, b: BorderSide) -> BorderSide {
        debug_assert!(Self::can_merge(a, b));
        let a_is_none = a.style == BorderStyle::None && a.width == 0.0;
        let b_is_none = b.style == BorderStyle::None && b.width == 0.0;
        if a_is_none && b_is_none {
            return BorderSide::NONE;
        }
        if a_is_none {
            return b;
        }
        if b_is_none {
            return a;
        }
        debug_assert_eq!(a.color, b.color);
        debug_assert_eq!(a.style, b.style);
        BorderSide {
            color: a.color, // == b.color
            width: a.width + b.width,
            stroke_align: a.stroke_align.max(b.stroke_align),
            style: a.style, // == b.style
        }
    }

    /// Creates a copy of this border but with the given fields replaced with the new values.
    pub fn copy_with(
        &self,
        color: Option<Color>,
        width: Option<f64>,
        style: Option<BorderStyle>,
        stroke_align: Option<f64>,
    ) -> BorderSide {
        BorderSide {
            color: color.unwrap_or(self.color),
            width: width.unwrap_or(self.width),
            style: style.unwrap_or(self.style),
            stroke_align: stroke_align.unwrap_or(self.stroke_align),
        }
    }

    /// Creates a copy of this border side description but with the width scaled
    /// by the factor `t`.
    ///
    /// Since a zero width is normally painted as a hairline width rather than no
    /// border at all, the zero factor is special-cased to instead change the
    /// style to [`BorderStyle::None`].
    ///
    /// Negative values are treated like zero.
    pub fn scale(&self, t: f64) -> BorderSide {
        BorderSide {
            color: self.color,
            width: (self.width * t).max(0.0),
            style: if t <= 0.0 {
                BorderStyle::None
            } else {
                self.style
            },
            stroke_align: Self::STROKE_ALIGN_INSIDE,
        }
    }

    /// Create a [`Paint`] object that, if used to stroke a line, will draw the
    /// line in this border's style.
    ///
    /// The [`stroke_align`](Self::stroke_align) property is not reflected in the
    /// [`Paint`]; consumers must implement that directly by inflating or
    /// deflating their region appropriately.
    pub fn to_paint(&self) -> Paint {
        match self.style {
            BorderStyle::Solid => Paint {
                color: self.color.into(),
                style: PaintStyle::Stroke(Stroke::new(self.width as f32)),
                ..Paint::default()
            },
            BorderStyle::None => Paint {
                color: Color::new(0x00000000).into(),
                style: PaintStyle::Stroke(Stroke::new(0.0)),
                ..Paint::default()
            },
        }
    }

    /// Whether the two given [`BorderSide`]s can be merged using [`merge`](Self::merge).
    ///
    /// Two sides can be merged if one or both are zero-width with
    /// [`BorderStyle::None`], or if they both have the same color and style.
    pub fn can_merge(a: BorderSide, b: BorderSide) -> bool {
        if (a.style == BorderStyle::None && a.width == 0.0)
            || (b.style == BorderStyle::None && b.width == 0.0)
        {
            return true;
        }
        a.style == b.style && a.color == b.color
    }

    /// Linearly interpolate between two border sides.
    pub fn lerp(a: BorderSide, b: BorderSide, t: f64) -> BorderSide {
        if a == b {
            return a;
        }
        if t == 0.0 {
            return a;
        }
        if t == 1.0 {
            return b;
        }
        let width = lerp_double(Some(a.width), Some(b.width), t).unwrap();
        if width < 0.0 {
            return BorderSide::NONE;
        }
        if a.style == b.style && a.stroke_align == b.stroke_align {
            return BorderSide {
                color: Color::lerp(Some(a.color), Some(b.color), t).unwrap(),
                width,
                style: a.style,               // == b.style
                stroke_align: a.stroke_align, // == b.stroke_align
            };
        }
        let color_a = match a.style {
            BorderStyle::Solid => a.color,
            BorderStyle::None => a.color.with_alpha(0x00),
        };
        let color_b = match b.style {
            BorderStyle::Solid => b.color,
            BorderStyle::None => b.color.with_alpha(0x00),
        };
        if a.stroke_align != b.stroke_align {
            return BorderSide {
                color: Color::lerp(Some(color_a), Some(color_b), t).unwrap(),
                width,
                stroke_align: lerp_double(Some(a.stroke_align), Some(b.stroke_align), t).unwrap(),
                style: BorderStyle::Solid,
            };
        }
        BorderSide {
            color: Color::lerp(Some(color_a), Some(color_b), t).unwrap(),
            width,
            stroke_align: a.stroke_align, // == b.stroke_align
            style: BorderStyle::Solid,
        }
    }

    /// Get the amount of the stroke width that lies inside of the [`BorderSide`].
    pub fn stroke_inset(&self) -> f64 {
        self.width * (1.0 - (1.0 + self.stroke_align) / 2.0)
    }

    /// Get the amount of the stroke width that lies outside of the [`BorderSide`].
    pub fn stroke_outset(&self) -> f64 {
        self.width * (1.0 + self.stroke_align) / 2.0
    }

    /// The offset of the stroke, taking into account the stroke alignment.
    pub fn stroke_offset(&self) -> f64 {
        self.width * self.stroke_align
    }
}

impl Default for BorderSide {
    fn default() -> BorderSide {
        BorderSide::new(
            Color::new(0xFF000000),
            1.0,
            BorderStyle::Solid,
            Self::STROKE_ALIGN_INSIDE,
        )
    }
}

impl Debug for BorderSide {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "BorderSide(color: {:?}, width: {:.1})",
            self.color, self.width
        )
    }
}

/// Paints a border around the given rectangle on the canvas.
///
/// The four sides can be independently specified. They are painted in the order
/// top, right, bottom, left. This is only notable if the widths of the borders
/// and the size of the given rectangle are such that the border sides will
/// overlap each other. No effort is made to optimize the rendering of uniform
/// borders (where all the borders have the same configuration); to render a
/// uniform border, consider using `Canvas.draw_rect` directly.
pub fn paint_border(
    canvas: &mut Canvas,
    rect: Rect,
    top: BorderSide,
    right: BorderSide,
    bottom: BorderSide,
    left: BorderSide,
) {
    // We draw the borders as filled shapes, unless the borders are hairline
    // borders, in which case we use a stroke, with the stroke width specified
    // here.
    let mut paint = Paint {
        style: PaintStyle::Stroke(Stroke::new(0.0)),
        ..Paint::default()
    };

    match top.style {
        BorderStyle::Solid => {
            paint.color = top.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.left, rect.top));
            path.line_to(Offset::new(rect.right, rect.top));
            if top.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(rect.right - right.width, rect.top + top.width));
                path.line_to(Offset::new(rect.left + left.width, rect.top + top.width));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }

    match right.style {
        BorderStyle::Solid => {
            paint.color = right.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.right, rect.top));
            path.line_to(Offset::new(rect.right, rect.bottom));
            if right.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(
                    rect.right - right.width,
                    rect.bottom - bottom.width,
                ));
                path.line_to(Offset::new(rect.right - right.width, rect.top + top.width));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }

    match bottom.style {
        BorderStyle::Solid => {
            paint.color = bottom.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.right, rect.bottom));
            path.line_to(Offset::new(rect.left, rect.bottom));
            if bottom.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(
                    rect.left + left.width,
                    rect.bottom - bottom.width,
                ));
                path.line_to(Offset::new(
                    rect.right - right.width,
                    rect.bottom - bottom.width,
                ));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }

    match left.style {
        BorderStyle::Solid => {
            paint.color = left.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.left, rect.bottom));
            path.line_to(Offset::new(rect.left, rect.top));
            if left.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(rect.left + left.width, rect.top + top.width));
                path.line_to(Offset::new(
                    rect.left + left.width,
                    rect.bottom - bottom.width,
                ));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(color: Color, width: f64) -> BorderSide {
        BorderSide {
            color,
            width,
            ..BorderSide::default()
        }
    }

    #[test]
    fn merging() {
        let blue = BorderSide {
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        let blue2 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 2.0,
            ..BorderSide::default()
        };
        let green = BorderSide {
            color: Color::new(0xFF00FF00),
            ..BorderSide::default()
        };
        let green2 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 2.0,
            ..BorderSide::default()
        };
        let green3 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 3.0,
            ..BorderSide::default()
        };
        let green5 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 5.0,
            ..BorderSide::default()
        };
        let none = BorderSide {
            style: BorderStyle::None,
            ..BorderSide::default()
        };
        let none2 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 2.0,
            style: BorderStyle::None,
            ..BorderSide::default()
        };
        let none3 = BorderSide {
            style: BorderStyle::None,
            width: 3.0,
            ..BorderSide::default()
        };
        let side2 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        let side3 = BorderSide {
            width: 3.0,
            ..BorderSide::default()
        };
        let side5 = BorderSide {
            width: 5.0,
            ..BorderSide::default()
        };
        let solid = BorderSide::default();
        let yellow_none = BorderSide {
            style: BorderStyle::None,
            color: Color::new(0xFFFFFF00),
            width: 0.0,
            ..BorderSide::default()
        };

        assert!(BorderSide::can_merge(BorderSide::NONE, BorderSide::NONE));
        assert!(BorderSide::can_merge(BorderSide::NONE, side2));
        assert!(BorderSide::can_merge(BorderSide::NONE, yellow_none));
        assert!(!BorderSide::can_merge(green, blue));
        assert!(!BorderSide::can_merge(green2, blue2));
        assert!(BorderSide::can_merge(green2, green3));
        assert!(!BorderSide::can_merge(green2, none2));
        assert!(BorderSide::can_merge(none3, BorderSide::NONE));
        assert!(!BorderSide::can_merge(none3, side2));
        assert!(BorderSide::can_merge(none3, yellow_none));
        assert!(BorderSide::can_merge(side2, BorderSide::NONE));
        assert!(!BorderSide::can_merge(side2, none3));
        assert!(BorderSide::can_merge(side2, side3));
        assert!(BorderSide::can_merge(side2, yellow_none));
        assert!(BorderSide::can_merge(side3, side2));
        assert!(!BorderSide::can_merge(solid, none));
        assert!(BorderSide::can_merge(yellow_none, side2));
        assert!(BorderSide::can_merge(yellow_none, yellow_none));

        assert_eq!(
            BorderSide::merge(BorderSide::NONE, BorderSide::NONE),
            BorderSide::NONE
        );
        assert_eq!(BorderSide::merge(BorderSide::NONE, side2), side2);
        assert_eq!(
            BorderSide::merge(BorderSide::NONE, yellow_none),
            BorderSide::NONE
        );
        assert_eq!(BorderSide::merge(green2, green3), green5);
        assert_eq!(BorderSide::merge(none3, BorderSide::NONE), none3);
        assert_eq!(BorderSide::merge(none3, yellow_none), none3);
        assert_eq!(BorderSide::merge(side2, BorderSide::NONE), side2);
        assert_eq!(BorderSide::merge(side2, side3), side5);
        assert_eq!(BorderSide::merge(side2, yellow_none), side2);
        assert_eq!(BorderSide::merge(side3, side2), side5);
        assert_eq!(BorderSide::merge(yellow_none, side2), side2);
        assert_eq!(
            BorderSide::merge(yellow_none, yellow_none),
            BorderSide::NONE
        );
    }

    #[test]
    fn copy_with_and_scale() {
        let green2 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 2.0,
            ..BorderSide::default()
        };
        let blue3 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 3.0,
            ..BorderSide::default()
        };
        let blue2 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 2.0,
            ..BorderSide::default()
        };
        let green3 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 3.0,
            ..BorderSide::default()
        };
        let none2 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 2.0,
            style: BorderStyle::None,
            ..BorderSide::default()
        };
        assert_eq!(
            green2.copy_with(Some(Color::new(0xFF0000FF)), Some(3.0), None, None),
            blue3
        );
        assert_eq!(green2.copy_with(None, Some(3.0), None, None), green3);
        assert_eq!(
            green2.copy_with(Some(Color::new(0xFF0000FF)), None, None, None),
            blue2
        );
        assert_eq!(
            green2.copy_with(None, None, Some(BorderStyle::None), None),
            none2
        );

        let side3 = BorderSide {
            width: 3.0,
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        let side6 = BorderSide {
            width: 6.0,
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        let none = BorderSide {
            style: BorderStyle::None,
            width: 0.0,
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        assert_eq!(side3.scale(2.0), side6);
        assert_eq!(side6.scale(0.5), side3);
        assert_eq!(side6.scale(0.0), none);
        assert_eq!(side6.scale(-1.0), none);
        assert_eq!(none.scale(2.0), none);
    }

    #[test]
    fn to_paint_stroke_and_none() {
        let paint1 = BorderSide {
            width: 2.5,
            color: Color::new(0xFFFFFF00),
            ..BorderSide::default()
        }
        .to_paint();
        assert_eq!(paint1.style, PaintStyle::Stroke(Stroke::new(2.5)));
        assert_eq!(paint1.color, Color::new(0xFFFFFF00).into());
        assert_eq!(paint1.blend_mode, reveal_embedder::BlendMode::SrcOver);

        let paint2 = BorderSide {
            width: 2.5,
            color: Color::new(0xFFFFFF00),
            style: BorderStyle::None,
            ..BorderSide::default()
        }
        .to_paint();
        assert_eq!(paint2.style, PaintStyle::Stroke(Stroke::new(0.0)));
        assert_eq!(paint2.color, Color::new(0x00000000).into());
        assert_eq!(paint2.blend_mode, reveal_embedder::BlendMode::SrcOver);
    }

    #[test]
    fn lerp_identical_and_negative_width() {
        let border = BorderSide::default();
        assert_eq!(BorderSide::lerp(border, border, 0.5), border);

        let side0 = BorderSide {
            width: 0.0,
            ..BorderSide::default()
        };
        let side1 = BorderSide::default();
        let side2 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        assert_eq!(BorderSide::lerp(side2, side1, 10.0), BorderSide::NONE);
        assert_eq!(BorderSide::lerp(side1, side2, -10.0), BorderSide::NONE);
        assert_eq!(BorderSide::lerp(side0, side1, 2.0), side2);
        assert_eq!(BorderSide::lerp(side1, side0, 2.0), BorderSide::NONE);
        assert_eq!(BorderSide::lerp(side2, side1, 2.0), side0);
    }

    #[test]
    fn lerp_stroke_align() {
        let side0 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        let side1 = BorderSide {
            width: 2.0,
            stroke_align: BorderSide::STROKE_ALIGN_OUTSIDE,
            ..BorderSide::default()
        };
        assert_eq!(
            BorderSide::lerp(side0, side1, 0.0),
            BorderSide {
                width: 2.0,
                ..BorderSide::default()
            }
        );
        assert_eq!(
            BorderSide::lerp(side0, side1, 0.5),
            BorderSide {
                width: 2.0,
                stroke_align: BorderSide::STROKE_ALIGN_CENTER,
                ..BorderSide::default()
            }
        );
        assert_eq!(
            BorderSide::lerp(side0, side1, 1.0),
            BorderSide {
                width: 2.0,
                stroke_align: BorderSide::STROKE_ALIGN_OUTSIDE,
                ..BorderSide::default()
            }
        );

        let side2 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        let side3 = BorderSide {
            width: 2.0,
            stroke_align: BorderSide::STROKE_ALIGN_CENTER,
            ..BorderSide::default()
        };
        assert_eq!(
            BorderSide::lerp(side2, side3, 0.5),
            BorderSide {
                width: 2.0,
                stroke_align: -0.5,
                ..BorderSide::default()
            }
        );
    }

    #[test]
    fn paint_border_records_one_path_per_solid_side() {
        let mut canvas = Canvas::new();
        paint_border(
            &mut canvas,
            Rect::from_ltrb(0.0, 0.0, 10.0, 10.0),
            side(Color::new(0xFFFF0000), 1.0),
            side(Color::new(0xFF00FF00), 1.0),
            side(Color::new(0xFF0000FF), 1.0),
            side(Color::new(0xFFFFFF00), 1.0),
        );
        assert_eq!(canvas.build().draw_count(), 4);
    }
}

//! Flutter counterpart: `rendering/table_border.dart`.

use std::fmt;

use inset_embedder::{
    Canvas, Color, FillRule, Offset, Paint, PaintStyle, PathBuilder, Rect, Stroke,
};
use inset_painting::{
    BorderRadius, BorderSide, BorderStyle, EdgeInsets, draw_drrect, paint_border,
};

/// Border specification for `Table` widgets.
///
/// This is like `Border`, with the addition of two sides: the inner horizontal borders between
/// rows and the inner vertical borders between columns.
///
/// The sides are represented by [`BorderSide`] objects.
#[derive(Clone, Copy, PartialEq)]
pub struct TableBorder {
    /// The top side of this border.
    pub top: BorderSide,
    /// The right side of this border.
    pub right: BorderSide,
    /// The bottom side of this border.
    pub bottom: BorderSide,
    /// The left side of this border.
    pub left: BorderSide,
    /// The horizontal interior sides of this border.
    pub horizontal_inside: BorderSide,
    /// The vertical interior sides of this border.
    pub vertical_inside: BorderSide,
    /// The [`BorderRadius`] to use when painting the corners of this border.
    ///
    /// It is also applied to `DataTable`'s `Material`.
    pub border_radius: BorderRadius,
}

impl Default for TableBorder {
    fn default() -> TableBorder {
        TableBorder::new()
    }
}

impl TableBorder {
    /// Creates a border for a table.
    ///
    /// All the sides of the border default to [`BorderSide::NONE`]; Dart's named arguments are
    /// the setters.
    pub const fn new() -> TableBorder {
        TableBorder {
            top: BorderSide::NONE,
            right: BorderSide::NONE,
            bottom: BorderSide::NONE,
            left: BorderSide::NONE,
            horizontal_inside: BorderSide::NONE,
            vertical_inside: BorderSide::NONE,
            border_radius: BorderRadius::ZERO,
        }
    }

    /// Dart `TableBorder(top:)`.
    pub fn top(mut self, top: BorderSide) -> TableBorder {
        self.top = top;
        self
    }

    /// Dart `TableBorder(right:)`.
    pub fn right(mut self, right: BorderSide) -> TableBorder {
        self.right = right;
        self
    }

    /// Dart `TableBorder(bottom:)`.
    pub fn bottom(mut self, bottom: BorderSide) -> TableBorder {
        self.bottom = bottom;
        self
    }

    /// Dart `TableBorder(left:)`.
    pub fn left(mut self, left: BorderSide) -> TableBorder {
        self.left = left;
        self
    }

    /// Dart `TableBorder(horizontalInside:)`.
    pub fn horizontal_inside(mut self, horizontal_inside: BorderSide) -> TableBorder {
        self.horizontal_inside = horizontal_inside;
        self
    }

    /// Dart `TableBorder(verticalInside:)`.
    pub fn vertical_inside(mut self, vertical_inside: BorderSide) -> TableBorder {
        self.vertical_inside = vertical_inside;
        self
    }

    /// Dart `TableBorder(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> TableBorder {
        self.border_radius = border_radius;
        self
    }

    /// A uniform border with all sides the same color and width.
    ///
    /// Dart's `TableBorder.all` defaults the sides to black solid borders, one logical pixel
    /// wide, with no radius.
    pub fn all(
        color: Color,
        width: f64,
        style: BorderStyle,
        border_radius: BorderRadius,
    ) -> TableBorder {
        let side = BorderSide::new(color, width, style, BorderSide::STROKE_ALIGN_INSIDE);
        TableBorder {
            top: side,
            right: side,
            bottom: side,
            left: side,
            horizontal_inside: side,
            vertical_inside: side,
            border_radius,
        }
    }

    /// Creates a border for a table where all the interior sides use the same styling and all
    /// the exterior sides use the same styling.
    pub fn symmetric(
        inside: BorderSide,
        outside: BorderSide,
        border_radius: BorderRadius,
    ) -> TableBorder {
        TableBorder {
            top: outside,
            right: outside,
            bottom: outside,
            left: outside,
            horizontal_inside: inside,
            vertical_inside: inside,
            border_radius,
        }
    }

    /// The widths of the sides of this border represented as an [`EdgeInsets`].
    ///
    /// This can be used, for example, with a `Padding` widget to inset a box by the size of
    /// these borders.
    pub fn dimensions(&self) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            self.left.width,
            self.top.width,
            self.right.width,
            self.bottom.width,
        )
    }

    /// Whether all the sides of the border (outside and inside) are identical. Uniform borders
    /// are typically more efficient to paint.
    pub fn is_uniform(&self) -> bool {
        self.all_sides_match(|side| side.color)
            && self.all_sides_match(|side| side.width)
            && self.all_sides_match(|side| side.style)
    }

    /// Whether all the outer sides of the border are identical.
    fn outer_border_is_uniform(&self) -> bool {
        self.outer_sides_match(|side| side.color)
            && self.outer_sides_match(|side| side.width)
            && self.outer_sides_match(|side| side.style)
    }

    fn all_sides_match<T: PartialEq>(&self, selector: impl Fn(&BorderSide) -> T) -> bool {
        let top_value = selector(&self.top);

        selector(&self.right) == top_value
            && selector(&self.bottom) == top_value
            && selector(&self.left) == top_value
            && selector(&self.horizontal_inside) == top_value
            && selector(&self.vertical_inside) == top_value
    }

    fn outer_sides_match<T: PartialEq>(&self, selector: impl Fn(&BorderSide) -> T) -> bool {
        let top_value = selector(&self.top);

        selector(&self.right) == top_value
            && selector(&self.bottom) == top_value
            && selector(&self.left) == top_value
    }

    /// Returns the set of distinct visible colors from the outer border sides.
    ///
    /// Only includes colors from border sides that are not [`BorderStyle::None`].
    fn distinct_visible_outer_colors(&self) -> Vec<Color> {
        let mut colors: Vec<Color> = Vec::new();
        for side in [self.top, self.right, self.bottom, self.left] {
            if side.style != BorderStyle::None && !colors.contains(&side.color) {
                colors.push(side.color);
            }
        }
        colors
    }

    fn paint_table_border(&self, canvas: &mut Canvas, rect: Rect) {
        if self.outer_border_is_uniform() && self.border_radius != BorderRadius::ZERO {
            let outer = self.border_radius.to_rrect(rect);
            let inner = outer.deflate(self.top.width);
            let paint = Paint {
                color: self.top.color.into(),
                ..Paint::default()
            };
            draw_drrect(canvas, outer, inner, &paint);
            return;
        }

        let visible_colors = self.distinct_visible_outer_colors();
        if visible_colors.len() == 1 && self.border_radius != BorderRadius::ZERO {
            let visible = |side: BorderSide| match side.style {
                BorderStyle::None => BorderSide::NONE,
                BorderStyle::Solid => side,
            };
            TableBorder::paint_non_uniform_border_with_radius(
                canvas,
                rect,
                self.border_radius,
                visible_colors[0],
                visible(self.top),
                visible(self.right),
                visible(self.bottom),
                visible(self.left),
            );
            return;
        }

        paint_border(canvas, rect, self.top, self.right, self.bottom, self.left);
    }

    /// Paints a non-uniform border with border radius support.
    ///
    /// This is similar to `BoxBorder.paintNonUniformBorder` but adapted for table borders.
    #[allow(clippy::too_many_arguments)]
    fn paint_non_uniform_border_with_radius(
        canvas: &mut Canvas,
        rect: Rect,
        border_radius: BorderRadius,
        color: Color,
        top: BorderSide,
        right: BorderSide,
        bottom: BorderSide,
        left: BorderSide,
    ) {
        let border_rect = border_radius.to_rrect(rect);
        let paint = Paint {
            color: color.into(),
            ..Paint::default()
        };

        let inner = EdgeInsets::from_ltrb(
            left.stroke_inset(),
            top.stroke_inset(),
            right.stroke_inset(),
            bottom.stroke_inset(),
        )
        .deflate_rrect(border_rect);

        let outer = EdgeInsets::from_ltrb(
            left.stroke_outset(),
            top.stroke_outset(),
            right.stroke_outset(),
            bottom.stroke_outset(),
        )
        .inflate_rrect(border_rect);

        draw_drrect(canvas, outer, inner, &paint);
    }

    /// Creates a copy of this border but with the widths scaled by the factor `t`.
    ///
    /// The `t` argument represents the multiplicand, or the position on the timeline for an
    /// interpolation from nothing to `self`, with 0.0 meaning that the object returned should
    /// be the nil variant of this object, 1.0 meaning that no change should be applied,
    /// returning `self` (or something equivalent to `self`), and other values meaning that the
    /// object should be multiplied by `t`. Negative values are treated like zero.
    ///
    /// Values for `t` are usually obtained from an `Animation<f64>`, such as an
    /// `AnimationController`.
    ///
    /// See also:
    ///
    ///  * [`BorderSide::scale`], which is used to implement this method.
    pub fn scale(&self, t: f64) -> TableBorder {
        TableBorder::new()
            .top(self.top.scale(t))
            .right(self.right.scale(t))
            .bottom(self.bottom.scale(t))
            .left(self.left.scale(t))
            .horizontal_inside(self.horizontal_inside.scale(t))
            .vertical_inside(self.vertical_inside.scale(t))
    }

    /// Linearly interpolate between two table borders.
    ///
    /// If a border is `None`, it is treated as having only [`BorderSide::NONE`] borders.
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning that the
    /// interpolation has not started, returning `a` (or something equivalent to `a`), 1.0
    /// meaning that the interpolation has finished, returning `b` (or something equivalent to
    /// `b`), and values in between meaning that the interpolation is at the relevant point on
    /// the timeline between `a` and `b`.
    pub fn lerp(a: Option<TableBorder>, b: Option<TableBorder>, t: f64) -> Option<TableBorder> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (Some(a), Some(b)) => Some(
                TableBorder::new()
                    .top(BorderSide::lerp(a.top, b.top, t))
                    .right(BorderSide::lerp(a.right, b.right, t))
                    .bottom(BorderSide::lerp(a.bottom, b.bottom, t))
                    .left(BorderSide::lerp(a.left, b.left, t))
                    .horizontal_inside(BorderSide::lerp(
                        a.horizontal_inside,
                        b.horizontal_inside,
                        t,
                    ))
                    .vertical_inside(BorderSide::lerp(a.vertical_inside, b.vertical_inside, t)),
            ),
        }
    }

    /// Paints the border around the given [`Rect`] on the given [`Canvas`], with the given
    /// rows and columns.
    ///
    /// Uniform borders are more efficient to paint than more complex borders.
    ///
    /// The `rows` argument specifies the vertical positions between the rows, relative to the
    /// given rectangle. For example, if the table contained two rows of height 100.0 each,
    /// then `rows` would contain a single value, 100.0, which is the vertical position between
    /// the two rows (relative to the top edge of `rect`).
    ///
    /// The `columns` argument specifies the horizontal positions between the columns, relative
    /// to the given rectangle. For example, if the table contained two columns of height 100.0
    /// each, then `columns` would contain a single value, 100.0, which is the vertical position
    /// between the two columns (relative to the left edge of `rect`).
    ///
    /// The [`vertical_inside`](Self::vertical_inside) border is only drawn if there are at
    /// least two columns. The [`horizontal_inside`](Self::horizontal_inside) border is only
    /// drawn if there are at least two rows. The horizontal borders are drawn after the
    /// vertical borders.
    ///
    /// The outer borders (in the order [`top`](Self::top), [`right`](Self::right),
    /// [`bottom`](Self::bottom), [`left`](Self::left), with [`left`](Self::left) above the
    /// others) are painted after the inner borders.
    ///
    /// The paint order is particularly notable in the case of partially-transparent borders.
    pub fn paint(&self, canvas: &mut Canvas, rect: Rect, rows: &[f64], columns: &[f64]) {
        // properties can't be null

        // arguments can't be null
        debug_assert!(rows.is_empty() || (rows[0] >= 0.0 && rows[rows.len() - 1] <= rect.height()));
        debug_assert!(
            columns.is_empty() || (columns[0] >= 0.0 && columns[columns.len() - 1] <= rect.width())
        );

        if !columns.is_empty() || !rows.is_empty() {
            let mut paint = Paint::default();

            if !columns.is_empty() {
                match self.vertical_inside.style {
                    BorderStyle::Solid => {
                        paint.color = self.vertical_inside.color.into();
                        paint.style =
                            PaintStyle::Stroke(Stroke::new(self.vertical_inside.width as f32));
                        let mut path = PathBuilder::new();
                        for &x in columns {
                            path.move_to(Offset::new(rect.left + x, rect.top));
                            path.line_to(Offset::new(rect.left + x, rect.bottom));
                        }
                        canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
                    }
                    BorderStyle::None => {}
                }
            }

            if !rows.is_empty() {
                match self.horizontal_inside.style {
                    BorderStyle::Solid => {
                        paint.color = self.horizontal_inside.color.into();
                        paint.style =
                            PaintStyle::Stroke(Stroke::new(self.horizontal_inside.width as f32));
                        let mut path = PathBuilder::new();
                        for &y in rows {
                            path.move_to(Offset::new(rect.left, rect.top + y));
                            path.line_to(Offset::new(rect.right, rect.top + y));
                        }
                        canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
                    }
                    BorderStyle::None => {}
                }
            }
        }

        self.paint_table_border(canvas, rect);
    }
}

impl fmt::Debug for TableBorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TableBorder({:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?})",
            self.top,
            self.right,
            self.bottom,
            self.left,
            self.horizontal_inside,
            self.vertical_inside,
            self.border_radius
        )
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::Radius;

    use super::*;

    fn side(width: f64, color: u32) -> BorderSide {
        BorderSide::new(
            Color::new(color),
            width,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_INSIDE,
        )
    }

    /// `table_border_test.dart`: "TableBorder constructor".
    #[test]
    fn the_sides_not_given_are_none() {
        let border = TableBorder::new()
            .left(BorderSide::default())
            .right(side(1.0, 0xFF00FF00))
            .vertical_inside(BorderSide::default());
        assert!(border.top == BorderSide::NONE);
        assert!(border.right == side(1.0, 0xFF00FF00));
        assert!(border.bottom == BorderSide::NONE);
        assert!(border.left == BorderSide::default());
        assert!(border.horizontal_inside == BorderSide::NONE);
        assert!(border.vertical_inside == BorderSide::default());
        assert!(border.dimensions() == EdgeInsets::symmetric(0.0, 1.0));
        assert!(!border.is_uniform());
        assert!(
            border.scale(2.0)
                == TableBorder::new()
                    .left(side(2.0, 0xFF000000))
                    .right(side(2.0, 0xFF00FF00))
                    .vertical_inside(side(2.0, 0xFF000000))
        );
    }

    /// `table_border_test.dart`: "TableBorder.all constructor" and "TableBorder.symmetric
    /// constructor".
    #[test]
    fn all_and_symmetric_fill_the_sides() {
        let all = TableBorder::all(
            Color::new(0xFF00FFFF),
            2.0,
            BorderStyle::Solid,
            BorderRadius::ZERO,
        );
        for each in [
            all.top,
            all.right,
            all.bottom,
            all.left,
            all.horizontal_inside,
            all.vertical_inside,
        ] {
            assert!(each == side(2.0, 0xFF00FFFF));
        }
        assert!(all.dimensions() == EdgeInsets::symmetric(2.0, 2.0));
        assert!(all.is_uniform());
        assert!(
            all.scale(0.5)
                == TableBorder::all(
                    Color::new(0xFF00FFFF),
                    1.0,
                    BorderStyle::Solid,
                    BorderRadius::ZERO
                )
        );

        let symmetric = TableBorder::symmetric(
            side(3.0, 0xFF000000),
            side(1.0, 0xFFFF0000),
            BorderRadius::ZERO,
        );
        assert!(symmetric.top == side(1.0, 0xFFFF0000));
        assert!(symmetric.horizontal_inside == side(3.0, 0xFF000000));
        assert!(symmetric.dimensions() == EdgeInsets::symmetric(1.0, 1.0));
        assert!(!symmetric.is_uniform());
        let gone = BorderSide::new(
            Color::new(0xFFFF0000),
            0.0,
            BorderStyle::None,
            BorderSide::STROKE_ALIGN_INSIDE,
        );
        assert!(
            symmetric.scale(0.0)
                == TableBorder::symmetric(BorderSide::NONE.scale(0.0), gone, BorderRadius::ZERO)
        );
    }

    /// `table_border_test.dart`: "TableBorder.lerp" and "TableBorder.lerp with nulls".
    #[test]
    fn lerp_scales_every_side() {
        let sides = [
            side(1.0, 0x00000001),
            side(2.0, 0x00000002),
            side(3.0, 0x00000003),
            side(4.0, 0x00000004),
            side(5.0, 0x00000005),
            side(6.0, 0x00000006),
        ];
        let scaled = |t: f64| {
            TableBorder::new()
                .top(sides[0].scale(t))
                .right(sides[1].scale(t))
                .bottom(sides[2].scale(t))
                .left(sides[3].scale(t))
                .horizontal_inside(sides[4].scale(t))
                .vertical_inside(sides[5].scale(t))
        };
        let (a, b, c) = (scaled(1.0), scaled(2.0), scaled(3.0));
        assert!(a.dimensions() == EdgeInsets::from_ltrb(4.0, 1.0, 2.0, 3.0));
        assert!(TableBorder::lerp(Some(a), Some(c), 0.5) == Some(b));
        assert!(TableBorder::lerp(Some(a), Some(b), 2.0) == Some(c));
        assert!(TableBorder::lerp(Some(b), Some(c), -1.0) == Some(a));

        let two = TableBorder::all(
            Color::new(0xFF000000),
            2.0,
            BorderStyle::Solid,
            BorderRadius::ZERO,
        );
        let one = TableBorder::all(
            Color::new(0xFF000000),
            1.0,
            BorderStyle::Solid,
            BorderRadius::ZERO,
        );
        assert!(TableBorder::lerp(Some(two), None, 0.5) == Some(one));
        assert!(TableBorder::lerp(None, Some(two), 0.5) == Some(one));
        assert!(TableBorder::lerp(None, None, 0.5).is_none());
    }

    /// `table_border_test.dart`: "TableBorder.all with a borderRadius".
    #[test]
    fn all_keeps_its_radius() {
        let radius = BorderRadius::all(Radius::circular(8.0));
        let border = TableBorder::all(Color::new(0xFF000000), 1.0, BorderStyle::Solid, radius);
        assert!(border.border_radius == radius);
    }
}

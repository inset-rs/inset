//! Flutter counterpart: `painting/clip.dart`.

use std::sync::Arc;

use inset_embedder::{
    Canvas, Clip, ClipOp, FillRule, Paint, Path, PathBuilder, RRect, RSuperellipse, Rect,
    rrect_radii_elliptical, rsuperellipse_radii_elliptical,
};

/// Clip utilities used by `PaintingContext`.
pub trait ClipContext {
    /// The canvas on which to paint.
    fn canvas(&mut self) -> &mut Canvas;

    /// Clip [`canvas`](Self::canvas) with [`Path`] according to [`Clip`] and then
    /// paint. The canvas is restored to the pre-clip status afterwards.
    ///
    /// `bounds` is the saveLayer bounds used for [`Clip::AntiAliasWithSaveLayer`].
    fn clip_path_and_paint(
        &mut self,
        path: &Arc<Path>,
        clip_behavior: Clip,
        bounds: Rect,
        painter: impl FnOnce(&mut Self),
    ) {
        clip_and_paint(
            self,
            |canvas, _do_anti_alias| {
                canvas.clip_path(path, FillRule::NonZero, ClipOp::Intersect);
            },
            clip_behavior,
            bounds,
            painter,
        );
    }

    /// Clip [`canvas`](Self::canvas) with a rounded rect and then paint. The
    /// canvas is restored to the pre-clip status afterwards.
    ///
    /// `bounds` is the saveLayer bounds used for [`Clip::AntiAliasWithSaveLayer`].
    fn clip_rrect_and_paint(
        &mut self,
        rrect: RRect,
        clip_behavior: Clip,
        bounds: Rect,
        painter: impl FnOnce(&mut Self),
    ) {
        clip_and_paint(
            self,
            |canvas, _do_anti_alias| {
                canvas.clip_rrect_radii_elliptical(
                    rrect.outer_rect(),
                    rrect_radii_elliptical(rrect),
                    ClipOp::Intersect,
                );
            },
            clip_behavior,
            bounds,
            painter,
        );
    }

    /// Clip [`canvas`](Self::canvas) with a rounded superellipse and then paint.
    /// The canvas is restored to the pre-clip status afterwards.
    ///
    /// `bounds` is the saveLayer bounds used for [`Clip::AntiAliasWithSaveLayer`].
    fn clip_r_superellipse_and_paint(
        &mut self,
        rse: RSuperellipse,
        clip_behavior: Clip,
        bounds: Rect,
        painter: impl FnOnce(&mut Self),
    ) {
        clip_and_paint(
            self,
            |canvas, _do_anti_alias| {
                let mut path = PathBuilder::new();
                path.rsuperellipse_radii(
                    rse.outer_rect().into(),
                    rsuperellipse_radii_elliptical(rse),
                );
                canvas.clip_path(&path.build(), FillRule::NonZero, ClipOp::Intersect);
            },
            clip_behavior,
            bounds,
            painter,
        );
    }

    /// Clip [`canvas`](Self::canvas) with a rect and then paint. The canvas is
    /// restored to the pre-clip status afterwards.
    ///
    /// `bounds` is the saveLayer bounds used for [`Clip::AntiAliasWithSaveLayer`].
    fn clip_rect_and_paint(
        &mut self,
        rect: Rect,
        clip_behavior: Clip,
        bounds: Rect,
        painter: impl FnOnce(&mut Self),
    ) {
        clip_and_paint(
            self,
            |canvas, _do_anti_alias| {
                canvas.clip_rect(rect, ClipOp::Intersect);
            },
            clip_behavior,
            bounds,
            painter,
        );
    }
}

fn clip_and_paint<C: ClipContext + ?Sized>(
    ctx: &mut C,
    canvas_clip_call: impl FnOnce(&mut Canvas, bool),
    clip_behavior: Clip,
    bounds: Rect,
    painter: impl FnOnce(&mut C),
) {
    ctx.canvas().save();
    match clip_behavior {
        Clip::None => {}
        Clip::HardEdge => canvas_clip_call(ctx.canvas(), false),
        Clip::AntiAlias => canvas_clip_call(ctx.canvas(), true),
        Clip::AntiAliasWithSaveLayer => {
            canvas_clip_call(ctx.canvas(), true);
            ctx.canvas()
                .save_layer(Some(bounds.into()), &Paint::default());
        }
    }
    painter(ctx);
    if clip_behavior == Clip::AntiAliasWithSaveLayer {
        ctx.canvas().restore();
    }
    ctx.canvas().restore();
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_embedder::Radius;

    struct TestClip {
        canvas: Canvas,
    }

    impl ClipContext for TestClip {
        fn canvas(&mut self) -> &mut Canvas {
            &mut self.canvas
        }
    }

    #[test]
    fn clip_rect_and_paint_balances_the_save_stack() {
        let mut ctx = TestClip {
            canvas: Canvas::new(),
        };
        assert_eq!(ctx.canvas.save_count(), 1);
        let bounds = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
        ctx.clip_rect_and_paint(bounds, Clip::AntiAlias, bounds, |ctx| {
            assert_eq!(ctx.canvas().save_count(), 2);
        });
        assert_eq!(ctx.canvas.save_count(), 1);

        ctx.clip_rect_and_paint(bounds, Clip::AntiAliasWithSaveLayer, bounds, |ctx| {
            assert_eq!(ctx.canvas().save_count(), 3);
        });
        assert_eq!(ctx.canvas.save_count(), 1);
    }

    #[test]
    fn clip_r_superellipse_and_paint_balances_the_save_stack() {
        let mut ctx = TestClip {
            canvas: Canvas::new(),
        };
        let bounds = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
        let rse = RSuperellipse::from_rect_and_radius(bounds, Radius::circular(2.0));
        ctx.clip_r_superellipse_and_paint(rse, Clip::AntiAlias, bounds, |ctx| {
            assert_eq!(ctx.canvas().save_count(), 2);
        });
        assert_eq!(ctx.canvas.save_count(), 1);
    }
}

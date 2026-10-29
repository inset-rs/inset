//! Whether a scene has contents of its own to draw: anything the framework paints beyond the holes
//! its native surfaces show through. Flutter's embedder asks the same of each part of a frame
//! before it renders that part (`EmbedderExternalView::HasEngineRenderedContents`, through
//! `DlOpSpy`), and renders none that draws nothing.
//!
//! The framework paints a surface's hole as a rectangle cleared where the surface is placed
//! (`RenderSurfaceView`), so a scene whose only draws are holes looks the same drawn or not: its
//! surfaces over the view's background. A cleared rectangle is such a hole only where it covers
//! exactly what the placement shows: under transforms that keep rectangles upright, inside
//! rectangular clips, and in no layer, which would composite the cleared pixels over what lies
//! beneath it. Anything else is contents, so every doubt is answered by drawing. The invariant a
//! later editor must keep: a scene answered as having no contents looks the same drawn or not.
//!
//! The answer costs nothing for a scene that paints: valo counts a picture's draws as it records
//! them, and a scene with more draws than surfaces has contents. Only a scene with no more draws
//! than surfaces, a handful, is walked.

use inset_embedder::{Picture, Scene};
use valo::{BlendMode, ClipOp, Op, Paint, Path};
use valo_geometry::{MatrixKind, PathElement};

/// Whether `scene` paints anything beyond the holes its surfaces show through.
pub(crate) fn has_rendered_contents(scene: &Scene) -> bool {
    let draws = scene.picture.draw_count() as usize;
    // Each surface placed has one hole at most: valo records no draw that is clipped away.
    draws > scene.surfaces.len() || (draws > 0 && !draws_only_holes(&scene.picture))
}

/// Whether every draw of `picture` is a hole, and nothing around one makes it more or less.
fn draws_only_holes(picture: &Picture) -> bool {
    picture.ops().iter().all(|op| match op {
        Op::Save | Op::Restore => true,
        Op::Transform(matrix) => matrix.kind() == MatrixKind::AxisAligned,
        Op::ClipPath { path, op, .. } => *op == ClipOp::Intersect && is_upright_rectangle(path),
        Op::DrawRect { paint, .. } => clears(paint),
        Op::DrawDisplayList { list, .. } => list.draw_count() == 0 || draws_only_holes(list),
        // A layer composites what is cleared in it over what lies beneath: no hole shows through.
        Op::SaveLayer { .. } => false,
        Op::DrawPath { .. } | Op::RRectBlur { .. } | Op::DrawImage { .. } | Op::GlyphRun { .. } => {
            false
        }
    })
}

/// Whether `paint` clears what it covers, with a sharp edge.
fn clears(paint: &Paint) -> bool {
    paint.blend_mode == BlendMode::Clear
        && paint.mask_blur.is_none()
        && paint.image_filter.is_none()
}

/// Whether `path` is one rectangle along the axes, as a rectangular clip records it.
fn is_upright_rectangle(path: &Path) -> bool {
    let elements: Vec<PathElement> = path.elements().collect();
    let [
        PathElement::MoveTo(a),
        PathElement::LineTo(b),
        PathElement::LineTo(c),
        PathElement::LineTo(d),
        rest @ ..,
    ] = elements.as_slice()
    else {
        return false;
    };
    rest.iter().all(|element| *element == PathElement::Close)
        && [(a, b), (b, c), (c, d), (d, a)]
            .iter()
            .all(|(from, to)| from.x == to.x || from.y == to.y)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use inset_embedder::test_support::idle_surface;
    use inset_embedder::{Canvas, Clip, Matrix4, Offset, RRect, Radius, Rect, SceneBuilder};
    use valo::PathBuilder;

    use super::*;

    /// Where the surface goes, in logical pixels.
    const SURFACE: Rect = Rect::from_ltwh(0.0, 0.0, 400.0, 300.0);

    /// What the framework records for a view that shows one surface, as a program's window does:
    /// the view's pixel ratio, the surface's hole painted where it is placed, under `around`.
    fn scene_of_a_surface(
        around: impl FnOnce(&mut SceneBuilder, &dyn Fn(&mut SceneBuilder)),
    ) -> Scene {
        let mut builder = SceneBuilder::new();
        builder.push_transform(&Matrix4::scale(2.0, 2.0));
        around(&mut builder, &|builder| {
            builder.add_picture(Offset::ZERO, &Arc::new(hole(SURFACE)), false, false);
            builder.add_surface(idle_surface(), SURFACE);
        });
        builder.pop();
        builder.build()
    }

    fn hole(rect: Rect) -> Picture {
        let mut canvas = Canvas::new();
        canvas.draw_rect(rect, &clearing());
        canvas.build()
    }

    fn clearing() -> Paint {
        Paint {
            blend_mode: BlendMode::Clear,
            ..Paint::default()
        }
    }

    fn nothing_around(builder: &mut SceneBuilder, surface: &dyn Fn(&mut SceneBuilder)) {
        surface(builder);
    }

    #[test]
    fn a_scene_that_only_shows_its_surfaces_has_no_contents_of_its_own() {
        assert!(!has_rendered_contents(&scene_of_a_surface(nothing_around)));
        assert!(!has_rendered_contents(&scene_of_a_surface(
            |builder, surface| {
                builder.push_offset(10.0, 20.0);
                builder.push_clip_rect(Rect::from_ltwh(0.0, 0.0, 200.0, 100.0), Clip::HardEdge);
                surface(builder);
                builder.pop();
                builder.pop();
            }
        )));
    }

    #[test]
    fn an_empty_scene_has_no_contents() {
        assert!(!has_rendered_contents(&SceneBuilder::new().build()));
    }

    #[test]
    fn anything_painted_beside_the_holes_is_contents() {
        let painted = scene_of_a_surface(|builder, surface| {
            let mut canvas = Canvas::new();
            canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), &Paint::default());
            builder.add_picture(Offset::ZERO, &Arc::new(canvas.build()), false, false);
            surface(builder);
        });
        assert!(has_rendered_contents(&painted));

        let mut builder = SceneBuilder::new();
        builder.add_picture(Offset::ZERO, &Arc::new(hole(SURFACE)), false, false);
        assert!(
            has_rendered_contents(&builder.build()),
            "a hole with no surface placed shows what is behind the window"
        );
    }

    #[test]
    fn a_hole_is_contents_where_it_would_not_be_what_its_placement_shows() {
        let rotated = scene_of_a_surface(|builder, surface| {
            builder.push_transform(&Matrix4::rotation(0.5));
            surface(builder);
            builder.pop();
        });
        let rounded = scene_of_a_surface(|builder, surface| {
            builder.push_clip_rrect(
                RRect::from_rect_and_radius(SURFACE, Radius::circular(12.0)),
                Clip::AntiAlias,
            );
            surface(builder);
            builder.pop();
        });
        let faded = scene_of_a_surface(|builder, surface| {
            builder.push_opacity(128, Offset::ZERO);
            surface(builder);
            builder.pop();
        });
        assert!(has_rendered_contents(&rotated), "a rotated hole");
        assert!(
            has_rendered_contents(&rounded),
            "a hole with rounded corners"
        );
        assert!(has_rendered_contents(&faded), "a hole in a layer");
    }

    #[test]
    fn only_a_rectangle_along_the_axes_is_upright() {
        let mut rectangle = PathBuilder::new();
        rectangle.rect(valo::Rect::new(0.0, 0.0, 40.0, 30.0));
        assert!(is_upright_rectangle(&rectangle.build()));

        let mut rounded = PathBuilder::new();
        rounded.rrect(valo::Rect::new(0.0, 0.0, 40.0, 30.0), 4.0);
        assert!(!is_upright_rectangle(&rounded.build()));

        let mut diamond = PathBuilder::new();
        diamond
            .move_to((20.0, 0.0))
            .line_to((40.0, 15.0))
            .line_to((20.0, 30.0))
            .line_to((0.0, 15.0))
            .close();
        assert!(!is_upright_rectangle(&diamond.build()));
    }
}

//! Where a frame puts one native surface, with everything the containers above it do to it:
//! Flutter's `EmbeddedViewParams` and its `MutatorsStack`. A scene's slices hand a host one for
//! each surface, and a surface frame's children are placed by them too, in the parent's buffer's
//! pixels.
//!
//! A host applies what its compositor can and reads the rest through the summaries here: a
//! compositor that only moves and clips rectangles reads [`SurfacePlacement::bounds`],
//! [`SurfacePlacement::scale`] and [`SurfacePlacement::visible_bounds`]. The invariant a later
//! editor must keep: the mutators are what the containers above the surface do, outermost first,
//! and nothing a host can work out from them is stored beside them.

use std::fmt;
use std::sync::Arc;

use crate::composition::Surface;
use crate::geometry::{Matrix4, RRect, Rect};
use crate::painting::Path;

/// Where one frame puts one native surface. The surface is its producer's own handle, so a host
/// reaches the native object behind it directly and moves it.
#[derive(Clone)]
pub struct SurfacePlacement {
    pub surface: Arc<dyn Surface>,
    /// The surface's rectangle, in the coordinates the transforms among `mutators` lead to.
    pub rect: Rect,
    /// What the containers above the surface do to it, the outermost first: Flutter's
    /// `MutatorsStack`. A view's scene begins with its root transform, which takes the
    /// framework's logical pixels to the view's physical ones.
    pub mutators: Vec<Mutator>,
}

/// One thing a container above a surface does to it: Flutter's `Mutator`. Each is in the
/// coordinates the transforms before it lead to.
#[derive(Clone, Debug, PartialEq)]
pub enum Mutator {
    Transform(Matrix4),
    /// Nothing of the surface shows outside the rectangle.
    ClipRect(Rect),
    /// Nothing of the surface shows outside the rounded rectangle.
    ClipRRect(RRect),
    /// Nothing of the surface shows outside the path, filled non-zero.
    ClipPath(Arc<Path>),
    /// The surface shows at this alpha, out of 255.
    Opacity(u8),
}

impl SurfacePlacement {
    /// Every transform above the surface in one: Flutter's `transformMatrix`, which takes `rect`'s
    /// coordinates to the view's pixels.
    pub fn transform(&self) -> Matrix4 {
        self.mutators
            .iter()
            .fold(Matrix4::IDENTITY, |transform, mutator| match mutator {
                Mutator::Transform(matrix) => transform.then(matrix),
                _ => transform,
            })
    }

    /// Every opacity above the surface multiplied, from 0 to 1.
    pub fn opacity(&self) -> f64 {
        self.mutators
            .iter()
            .fold(1.0, |opacity, mutator| match mutator {
                Mutator::Opacity(alpha) => opacity * f64::from(*alpha) / 255.0,
                _ => opacity,
            })
    }

    /// How many of the view's pixels one unit of the surface's own coordinates spans along its
    /// own horizontal: the scale of every transform above it, the root's device pixel ratio
    /// among them. A producer draws its buffers at the device pixel ratio, so a host shows a
    /// buffer at `scale / device_pixel_ratio` of its size.
    pub fn scale(&self) -> f64 {
        let m = self.transform().to_flutter_array();
        f64::from(m[0]).hypot(f64::from(m[1]))
    }

    /// The box around the surface under every transform above it, in the view's pixels.
    pub fn bounds(&self) -> Rect {
        bounds_under(&self.transform(), self.rect)
    }

    /// [`bounds`](Self::bounds) cut down to the bounds of every clip above the surface: what a
    /// compositor that clips by rectangles alone shows of it. An empty rectangle on the clip's
    /// nearest edge when nothing shows.
    pub fn visible_bounds(&self) -> Rect {
        let mut transform = Matrix4::IDENTITY;
        let mut visible = self.bounds();
        for mutator in &self.mutators {
            let clip_bounds = match mutator {
                Mutator::Transform(matrix) => {
                    transform = transform.then(matrix);
                    continue;
                }
                Mutator::ClipRect(rect) => *rect,
                Mutator::ClipRRect(rrect) => rrect.outer_rect(),
                Mutator::ClipPath(path) => rect_of(path.bounds()),
                Mutator::Opacity(_) => continue,
            };
            visible = visible_part_of(visible, bounds_under(&transform, clip_bounds));
        }
        visible
    }
}

/// Two placements are equal when they put the same surface, by pointer, in the same place under
/// the same mutators: what a host compares to tell a frame worth presenting from one it already
/// shows.
impl PartialEq for SurfacePlacement {
    fn eq(&self, other: &SurfacePlacement) -> bool {
        Arc::ptr_eq(&self.surface, &other.surface)
            && self.rect == other.rect
            && self.mutators == other.mutators
    }
}

impl fmt::Debug for SurfacePlacement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfacePlacement")
            .field("surface", &Arc::as_ptr(&self.surface).cast::<()>())
            .field("rect", &self.rect)
            .field("mutators", &self.mutators)
            .finish()
    }
}

/// The box around `rect` under `transform`.
fn bounds_under(transform: &Matrix4, rect: Rect) -> Rect {
    rect_of(transform.map_rect(&rect.into()))
}

fn rect_of(rect: crate::valo::Rect) -> Rect {
    Rect::from_ltwh(
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.width),
        f64::from(rect.height),
    )
}

/// What of `rect` lies inside `clip`; an empty rectangle on `clip`'s nearest edge when nothing
/// does.
fn visible_part_of(rect: Rect, clip: Rect) -> Rect {
    let overlap = rect.intersect(clip);
    Rect::from_ltrb(
        overlap.left,
        overlap.top,
        overlap.right.max(overlap.left),
        overlap.bottom.max(overlap.top),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::painting::PathBuilder;

    fn placement(rect: Rect, mutators: Vec<Mutator>) -> SurfacePlacement {
        SurfacePlacement {
            surface: super::super::idle_surface(),
            rect,
            mutators,
        }
    }

    /// A page at half its size on a display of two pixels a point: each unit of the page's own
    /// spans one of the view's pixels, so a buffer drawn at two pixels a point shows at half its
    /// size.
    #[test]
    fn a_placement_carries_the_scale_and_the_bounds_of_every_transform_above_it() {
        let page = placement(
            Rect::from_ltwh(0.0, 0.0, 390.0, 844.0),
            vec![
                Mutator::Transform(Matrix4::scale(2.0, 2.0)),
                Mutator::Transform(Matrix4::translation(10.0, 20.0)),
                Mutator::Transform(Matrix4::scale(0.5, 0.5)),
            ],
        );
        assert_eq!(page.scale(), 1.0);
        assert_eq!(page.bounds(), Rect::from_ltwh(20.0, 40.0, 390.0, 844.0));
        assert_eq!(page.visible_bounds(), page.bounds(), "nothing clips it");
    }

    #[test]
    fn a_turned_surface_is_bounded_by_the_box_around_it() {
        let turned = placement(
            Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            vec![Mutator::Transform(Matrix4::rotation(
                std::f32::consts::FRAC_PI_4,
            ))],
        );
        let bounds = turned.bounds();
        let half_diagonal = 50f64.sqrt();
        assert!((bounds.left + half_diagonal).abs() < 1e-4, "{bounds:?}");
        assert!((bounds.right - half_diagonal).abs() < 1e-4, "{bounds:?}");
        assert!((turned.scale() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn what_shows_is_cut_by_the_bounds_of_every_clip_under_its_own_transform() {
        let path = {
            let mut path = PathBuilder::new();
            path.rrect(Rect::from_ltwh(0.0, 0.0, 200.0, 20.0).into(), 4.0);
            path.build()
        };
        let clipped = placement(
            Rect::from_ltwh(-5.0, -5.0, 300.0, 300.0),
            vec![
                Mutator::Transform(Matrix4::scale(2.0, 2.0)),
                Mutator::ClipRect(Rect::from_ltwh(0.0, 0.0, 100.0, 50.0)),
                Mutator::Transform(Matrix4::translation(10.0, 10.0)),
                Mutator::ClipPath(path),
            ],
        );
        assert_eq!(clipped.bounds(), Rect::from_ltwh(10.0, 10.0, 600.0, 600.0));
        // The path's bounds, (10, 10) to (210, 30), cut by the rectangle's, (0, 0) to (100, 50),
        // at two pixels to a unit.
        assert_eq!(
            clipped.visible_bounds(),
            Rect::from_ltrb(20.0, 20.0, 200.0, 60.0)
        );
    }

    #[test]
    fn a_surface_outside_its_clip_shows_nothing() {
        let outside = placement(
            Rect::from_ltwh(200.0, 0.0, 50.0, 50.0),
            vec![Mutator::ClipRect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0))],
        );
        assert!(outside.visible_bounds().is_empty());
    }

    #[test]
    fn opacities_multiply() {
        let faded = placement(
            Rect::from_ltwh(0.0, 0.0, 1.0, 1.0),
            vec![
                Mutator::Opacity(51),
                Mutator::Opacity(255),
                Mutator::Opacity(51),
            ],
        );
        assert!((faded.opacity() - 0.04).abs() < 1e-9, "{}", faded.opacity());
        assert_eq!(placement(Rect::ZERO, Vec::new()).opacity(), 1.0);
    }

    #[test]
    fn a_placement_is_its_surface_its_rectangle_and_its_mutators() {
        let rect = Rect::from_ltwh(0.0, 0.0, 4.0, 4.0);
        let first = placement(rect, vec![Mutator::Opacity(10)]);
        assert_eq!(first.clone(), first);
        let elsewhere = SurfacePlacement {
            rect: Rect::from_ltwh(1.0, 0.0, 4.0, 4.0),
            ..first.clone()
        };
        assert_ne!(elsewhere, first);
        let fainter = SurfacePlacement {
            mutators: vec![Mutator::Opacity(9)],
            ..first.clone()
        };
        assert_ne!(fainter, first);
        let another_surface = placement(rect, vec![Mutator::Opacity(10)]);
        assert_ne!(
            another_surface, first,
            "the same rectangle is not the same surface"
        );
    }
}

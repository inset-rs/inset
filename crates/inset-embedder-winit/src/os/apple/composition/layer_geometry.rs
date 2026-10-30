//! Where one placement puts a surface's layers: the clip layer's frame and the shapes that mask it,
//! the content layer's size and transform.
//!
//! Core Animation applies what is above a surface as Flutter's macOS embedder does
//! (`FlutterMutatorView.mm`): every transform in one, on the content layer; the bounds of every
//! clip as the clip layer's frame, which masks to its bounds; and, where a clip is not an upright
//! rectangle, its shape as a mask, a rounded rectangle only where a corner cuts into what shows.
//! Several shapes mask in turn, each the mask of the one before, which Core Animation intersects.
//! The opacity above a surface is not the layers': the room the scene makes for the surface is
//! cleared by that opacity, and the surface shows whole through it, so a surface nothing lets
//! show is hidden and any other is opaque.
//!
//! The invariant a later editor must keep: every length here is in the parent layer's points, the
//! window's physical pixels over its scale, as a placement's are in pixels.

use std::sync::Arc;

use inset_embedder::{
    Matrix4, Mutator, Path, PathBuilder, RRect, Rect, SurfacePlacement, rrect_radii_elliptical,
};
use objc2_core_foundation::{CFRetained, CGFloat};
use objc2_core_graphics::CGMutablePath;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::CATransform3D;
use valo_geometry::{PathElement, Point};

/// Where one placement puts a surface's layers, in its parent layer's points.
pub(super) struct LayerGeometry {
    /// The clip layer's frame: the bounds of what shows.
    pub(super) clip_frame: NSRect,
    /// The shapes that clip the surface within the clip layer's bounds, in its points, the
    /// outermost first: rounded and path clips, and rectangles under a turn. Empty when the
    /// bounds are all of it.
    pub(super) clip_shapes: Vec<CFRetained<CGMutablePath>>,
    /// The content layer's bounds: the surface in its own coordinates.
    pub(super) content_size: NSSize,
    /// What takes the content layer's coordinates to the clip layer's: every transform above the
    /// surface, laid out from the content layer's top-left corner.
    pub(super) content_transform: CATransform3D,
}

impl LayerGeometry {
    /// The layers of a surface placed by `placement` in a window `window_scale` pixels to the
    /// point. A producer draws at the window's scale, so the transforms above the surface show
    /// what it drew larger or smaller as they scale it. `None` for a surface nothing lets show:
    /// shrunk to nothing, or at no opacity.
    pub(super) fn of(placement: &SurfacePlacement, window_scale: f64) -> Option<LayerGeometry> {
        let to_points = Matrix4::scale(window_scale.recip() as f32, window_scale.recip() as f32);
        let to_view_points = to_points.then(&placement.transform());
        if placement.opacity() <= 0.0 || to_view_points.invert().is_none() {
            return None;
        }
        let clip_frame = points_of(placement.visible_bounds(), window_scale);
        let to_clip_layer =
            Matrix4::translation(-clip_frame.origin.x as f32, -clip_frame.origin.y as f32)
                .then(&to_points);
        let rect = placement.rect;
        let content_transform = to_clip_layer
            .then(&placement.transform())
            .then(&Matrix4::translation(rect.left as f32, rect.top as f32));
        Some(LayerGeometry {
            clip_frame,
            clip_shapes: clip_shapes(placement, &to_clip_layer),
            content_size: NSSize::new(rect.width(), rect.height()),
            content_transform: core_animation_transform(&content_transform),
        })
    }
}

/// The clips above `placement` whose shape its clip layer's bounds do not make, in the clip
/// layer's points, to which `to_clip_layer` takes the view's pixels.
fn clip_shapes(
    placement: &SurfacePlacement,
    to_clip_layer: &Matrix4,
) -> Vec<CFRetained<CGMutablePath>> {
    let visible = placement.visible_bounds();
    let mut transform = Matrix4::IDENTITY;
    let mut shapes = Vec::new();
    for mutator in &placement.mutators {
        let shape = match mutator {
            Mutator::Transform(matrix) => {
                transform = transform.then(matrix);
                continue;
            }
            Mutator::ClipRect(rect) => {
                if keeps_rectangles_upright(&transform) {
                    continue;
                }
                rect_path(*rect)
            }
            Mutator::ClipRRect(rrect) => {
                if !a_corner_cuts_into(*rrect, &transform, visible) {
                    continue;
                }
                rrect_path(*rrect)
            }
            Mutator::ClipPath(path) => Arc::clone(path),
            Mutator::Opacity(_) => continue,
        };
        shapes.push(core_graphics_path(&shape, &to_clip_layer.then(&transform)));
    }
    shapes
}

/// Whether `transform` takes an upright rectangle to an upright rectangle: it scales and moves,
/// and turns, shears and foreshortens nothing.
fn keeps_rectangles_upright(transform: &Matrix4) -> bool {
    let m = transform.to_flutter_array();
    transform.is_affine() && m[1] == 0.0 && m[4] == 0.0
}

/// Whether a rounded corner of `rrect`, under `transform`, cuts into `visible`, in the view's
/// pixels. Under a transform that turns it, the shape is always needed.
fn a_corner_cuts_into(rrect: RRect, transform: &Matrix4, visible: Rect) -> bool {
    let Some(inverse) = transform.invert() else {
        return true;
    };
    if !keeps_rectangles_upright(transform) {
        return true;
    }
    round_rect_corner_intersects(rrect, rect_of(inverse.map_rect(&visible.into())))
}

/// Whether `rect` reaches outside one of `rrect`'s rounded corners: Flutter's
/// `RoundRectCornerIntersects`, `rect` lying within `rrect`'s bounds.
fn round_rect_corner_intersects(rrect: RRect, rect: Rect) -> bool {
    let corners = [
        // Each corner of `rect`, the inner corner of the rounded one it may cut, its radii, and
        // the quadrant of that inner corner the rounded one lies in.
        (
            (rect.left, rect.top),
            (
                rrect.left + rrect.tl_radius_x,
                rrect.top + rrect.tl_radius_y,
            ),
            (rrect.tl_radius_x, rrect.tl_radius_y),
            (-1.0, -1.0),
        ),
        (
            (rect.right, rect.top),
            (
                rrect.right - rrect.tr_radius_x,
                rrect.top + rrect.tr_radius_y,
            ),
            (rrect.tr_radius_x, rrect.tr_radius_y),
            (1.0, -1.0),
        ),
        (
            (rect.left, rect.bottom),
            (
                rrect.left + rrect.bl_radius_x,
                rrect.bottom - rrect.bl_radius_y,
            ),
            (rrect.bl_radius_x, rrect.bl_radius_y),
            (-1.0, 1.0),
        ),
        (
            (rect.right, rect.bottom),
            (
                rrect.right - rrect.br_radius_x,
                rrect.bottom - rrect.br_radius_y,
            ),
            (rrect.br_radius_x, rrect.br_radius_y),
            (1.0, 1.0),
        ),
    ];
    corners
        .into_iter()
        .any(|((x, y), (inner_x, inner_y), radius, (sign_x, sign_y))| {
            let relative = ((x - inner_x) * sign_x, (y - inner_y) * sign_y);
            relative.0 > 0.0 && relative.1 > 0.0 && !inside_ellipse(relative, radius)
        })
}

/// Whether `point` lies inside the ellipse of `radius` centred at the origin; nothing lies inside
/// a corner with no radius, which the rectangle's own edges make.
fn inside_ellipse((x, y): (f64, f64), (radius_x, radius_y): (f64, f64)) -> bool {
    if radius_x <= 0.0 || radius_y <= 0.0 {
        return true;
    }
    (x * x) / (radius_x * radius_x) + (y * y) / (radius_y * radius_y) < 1.0
}

fn rect_path(rect: Rect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rect(rect.into());
    path.build()
}

fn rrect_path(rrect: RRect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rrect_radii_elliptical(rrect.outer_rect(), rrect_radii_elliptical(rrect));
    path.build()
}

/// `path` under `transform`, as Core Graphics holds a path. A conic, which Core Graphics has not,
/// becomes the cubic that runs closest to it, as a rounded rectangle's quarter ellipses do in
/// Flutter's macOS embedder.
fn core_graphics_path(path: &Path, transform: &Matrix4) -> CFRetained<CGMutablePath> {
    let mut transformed = PathBuilder::new();
    transformed.append(path, transform);
    let transformed = transformed.build();
    let core_graphics = CGMutablePath::new();
    let target = Some(&*core_graphics);
    let no_transform = std::ptr::null();
    let (mut current, mut contour_start) = (Point::ZERO, Point::ZERO);
    for element in transformed.elements() {
        // SAFETY: each call adds to a path made above and owned here, with no transform.
        unsafe {
            match element {
                PathElement::MoveTo(to) => {
                    CGMutablePath::move_to_point(target, no_transform, x(to), y(to));
                    contour_start = to;
                }
                PathElement::LineTo(to) => {
                    CGMutablePath::add_line_to_point(target, no_transform, x(to), y(to));
                }
                PathElement::QuadTo(control, to) => CGMutablePath::add_quad_curve_to_point(
                    target,
                    no_transform,
                    x(control),
                    y(control),
                    x(to),
                    y(to),
                ),
                PathElement::ConicTo(control, to, weight) => {
                    let [first, second] = cubic_controls_of_conic(current, control, to, weight);
                    CGMutablePath::add_curve_to_point(
                        target,
                        no_transform,
                        x(first),
                        y(first),
                        x(second),
                        y(second),
                        x(to),
                        y(to),
                    );
                }
                PathElement::CubicTo(first, second, to) => CGMutablePath::add_curve_to_point(
                    target,
                    no_transform,
                    x(first),
                    y(first),
                    x(second),
                    y(second),
                    x(to),
                    y(to),
                ),
                PathElement::Close => CGMutablePath::close_subpath(target),
            }
        }
        current = match element {
            PathElement::MoveTo(to)
            | PathElement::LineTo(to)
            | PathElement::QuadTo(_, to)
            | PathElement::ConicTo(_, to, _)
            | PathElement::CubicTo(_, _, to) => to,
            PathElement::Close => contour_start,
        };
    }
    core_graphics
}

/// The control points of the cubic from `from` to `to` closest to the conic with `control` and
/// `weight`: each a share of the way to the conic's control point, which for a quarter ellipse is
/// the usual 0.5523.
fn cubic_controls_of_conic(from: Point, control: Point, to: Point, weight: f32) -> [Point; 2] {
    let share = 4.0 * weight / (3.0 * (1.0 + weight));
    let towards = |start: Point| {
        Point::new(
            start.x + share * (control.x - start.x),
            start.y + share * (control.y - start.y),
        )
    };
    [towards(from), towards(to)]
}

fn x(point: Point) -> CGFloat {
    CGFloat::from(point.x)
}

fn y(point: Point) -> CGFloat {
    CGFloat::from(point.y)
}

/// `matrix` as Core Animation holds a transform: the same sixteen numbers, in the same order, as
/// Flutter's macOS embedder converts one, a point a row there and a column here.
fn core_animation_transform(matrix: &Matrix4) -> CATransform3D {
    let m = matrix.to_flutter_array().map(CGFloat::from);
    CATransform3D {
        m11: m[0],
        m12: m[1],
        m13: m[2],
        m14: m[3],
        m21: m[4],
        m22: m[5],
        m23: m[6],
        m24: m[7],
        m31: m[8],
        m32: m[9],
        m33: m[10],
        m34: m[11],
        m41: m[12],
        m42: m[13],
        m43: m[14],
        m44: m[15],
    }
}

/// `rect`, in a window's physical pixels, in points: `scale` pixels each.
pub(super) fn points_of(rect: Rect, scale: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.left / scale, rect.top / scale),
        NSSize::new(rect.width() / scale, rect.height() / scale),
    )
}

fn rect_of(rect: valo::Rect) -> Rect {
    Rect::from_ltwh(
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.width),
        f64::from(rect.height),
    )
}

#[cfg(test)]
mod tests {
    use inset_embedder::{Radius, Surface, SurfaceFrame, SurfaceParent};
    use objc2_core_graphics::CGPath;

    use super::*;

    #[test]
    fn a_placement_in_physical_pixels_is_a_frame_in_points() {
        let frame = points_of(Rect::from_ltwh(240.0, 180.0, 800.0, 600.0), 2.0);
        assert_eq!(frame.origin, NSPoint::new(120.0, 90.0));
        assert_eq!(frame.size, NSSize::new(400.0, 300.0));
    }

    /// The surface keeps its own coordinates, and the transform takes them to the window's
    /// points: unscaled at two pixels a point a point of the surface is a point of the window.
    #[test]
    fn a_surface_keeps_its_own_size_and_is_scaled_by_its_transform() {
        let geometry = |scale: f32| {
            LayerGeometry::of(
                &placement(
                    Rect::from_ltwh(0.0, 0.0, 400.0, 300.0),
                    vec![Mutator::Transform(Matrix4::scale(scale, scale))],
                ),
                2.0,
            )
            .unwrap()
        };
        let whole = geometry(2.0);
        assert_eq!(whole.content_size, NSSize::new(400.0, 300.0));
        assert_eq!(
            (whole.content_transform.m11, whole.content_transform.m22),
            (1.0, 1.0)
        );
        let half = geometry(1.0);
        assert_eq!(half.content_size, NSSize::new(400.0, 300.0));
        assert_eq!(
            (half.content_transform.m11, half.content_transform.m22),
            (0.5, 0.5)
        );
    }

    /// The whole transform reaches the layer, from the clip layer's corner: a surface turned a
    /// quarter about its own corner and moved right lies to the left of where it is moved to.
    #[test]
    fn the_whole_transform_reaches_the_content_layer() {
        let turned = LayerGeometry::of(
            &placement(
                Rect::from_ltwh(0.0, 0.0, 200.0, 100.0),
                vec![
                    Mutator::Transform(Matrix4::translation(300.0, 0.0)),
                    Mutator::Transform(Matrix4::rotation(std::f32::consts::FRAC_PI_2)),
                ],
            ),
            2.0,
        )
        .unwrap();
        assert_eq!(
            turned.clip_frame.size,
            NSSize::new(50.0, 100.0),
            "the turned surface's bounds"
        );
        let t = turned.content_transform;
        assert!(t.m11.abs() < 1e-6 && (t.m12 - 0.5).abs() < 1e-6, "{t:?}");
        assert!((t.m21 + 0.5).abs() < 1e-6 && t.m22.abs() < 1e-6, "{t:?}");
        assert!(
            (t.m41 - 50.0).abs() < 1e-4 && t.m42.abs() < 1e-4,
            "the corner the surface turns about, at the clip layer's right edge: {t:?}"
        );
    }

    #[test]
    fn a_surface_nothing_lets_show_has_no_layers() {
        let rect = Rect::from_ltwh(0.0, 0.0, 10.0, 10.0);
        assert!(LayerGeometry::of(&placement(rect, vec![Mutator::Opacity(0)]), 2.0).is_none());
        assert!(
            LayerGeometry::of(
                &placement(rect, vec![Mutator::Transform(Matrix4::scale(0.0, 1.0))]),
                2.0
            )
            .is_none()
        );
        assert!(LayerGeometry::of(&placement(rect, vec![Mutator::Opacity(1)]), 2.0).is_some());
    }

    #[test]
    fn an_upright_rectangle_clips_by_the_bounds_alone_and_a_turned_one_by_its_shape() {
        let clip = Mutator::ClipRect(Rect::from_ltwh(0.0, 0.0, 50.0, 50.0));
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
        let upright = LayerGeometry::of(&placement(rect, vec![clip.clone()]), 1.0).unwrap();
        assert!(upright.clip_shapes.is_empty());
        let turned = LayerGeometry::of(
            &placement(rect, vec![Mutator::Transform(Matrix4::rotation(0.3)), clip]),
            1.0,
        )
        .unwrap();
        assert_eq!(turned.clip_shapes.len(), 1);
    }

    /// A rounded rectangle masks only where a corner reaches what shows, in the clip layer's
    /// points: a surface in its middle needs the bounds alone.
    #[test]
    fn a_rounded_clip_masks_only_where_a_corner_cuts_into_the_surface() {
        let card = Mutator::ClipRRect(RRect::from_rect_and_radius(
            Rect::from_ltwh(20.0, 20.0, 100.0, 100.0),
            Radius::circular(10.0),
        ));
        let filling = LayerGeometry::of(
            &placement(
                Rect::from_ltwh(20.0, 20.0, 100.0, 100.0),
                vec![card.clone()],
            ),
            2.0,
        )
        .unwrap();
        let [shape] = filling.clip_shapes.as_slice() else {
            panic!("one shape: {}", filling.clip_shapes.len());
        };
        let bounds = CGPath::bounding_box(Some(shape));
        assert!(
            bounds.origin.x.abs() < 1e-6
                && bounds.origin.y.abs() < 1e-6
                && (bounds.size.width - 50.0).abs() < 1e-4,
            "the card in the clip layer's points: {bounds:?}"
        );
        let inside = LayerGeometry::of(
            &placement(Rect::from_ltwh(40.0, 40.0, 60.0, 60.0), vec![card]),
            2.0,
        )
        .unwrap();
        assert!(inside.clip_shapes.is_empty());
    }

    #[test]
    fn a_path_clip_always_masks_and_shapes_stay_outermost_first() {
        let triangle = {
            let mut path = PathBuilder::new();
            path.move_to((0.0, 0.0))
                .line_to((100.0, 0.0))
                .line_to((0.0, 100.0))
                .close();
            path.build()
        };
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
        let geometry = LayerGeometry::of(
            &placement(
                rect,
                vec![
                    Mutator::ClipPath(triangle),
                    Mutator::Transform(Matrix4::rotation(0.1)),
                    Mutator::ClipRect(rect),
                ],
            ),
            1.0,
        )
        .unwrap();
        assert_eq!(geometry.clip_shapes.len(), 2);
        let first = CGPath::bounding_box(Some(&geometry.clip_shapes[0]));
        assert!(
            (first.size.width - 100.0).abs() < 1e-3,
            "the triangle first: {first:?}"
        );
    }

    fn placement(rect: Rect, mutators: Vec<Mutator>) -> SurfacePlacement {
        SurfacePlacement {
            surface: Arc::new(NoSurface),
            rect,
            mutators,
        }
    }

    /// A surface a placement names and the geometry never reads.
    struct NoSurface;

    impl Surface for NoSurface {
        fn present(&self, _frame: SurfaceFrame, _released: Box<dyn FnOnce() + Send + Sync>) {}
    }

    impl SurfaceParent for NoSurface {
        fn open_child(&self, _opaque: bool) -> Arc<dyn Surface> {
            Arc::new(NoSurface)
        }
    }
}

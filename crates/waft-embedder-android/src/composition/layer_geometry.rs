//! Where one placement puts a composition layer's container surface control in its parent's:
//! moved, scaled and cropped, which is all a surface control is placed by; and where a buffer lies
//! in its layer: a producer's filling it from its origin, and a system's frame turned, shown whole
//! within the layer's box and centred in it.
//!
//! The NDK places a surface control by an integer position, one scale and an axis-aligned crop in
//! its own pixels (API 31), and has no arbitrary matrix and no rounded or path clip, so a layer is
//! placed as `Placement`'s summaries say for "a compositor that only moves and clips rectangles":
//! its box under every transform above it, the scale of those transforms, and what of the box the
//! bounds of every clip above it leave. The opacity above a layer is not the surface control's: the
//! room the scene makes for it is cleared by that opacity, and the layer shows whole through it,
//! so a layer nothing lets show is hidden and any other is shown.
//!
//! The invariant a later editor must keep: a container's own pixels are its buffer's, `pixel_ratio`
//! of them to one of the layer's units, as a producer draws its buffers at the view's device pixel
//! ratio, and its children are placed in them; the parent's pixels are the view's for a layer a
//! scene places and the parent's buffer's for a child.

use waft_embedder::Placement;

use crate::buffer_transform::BufferTransform;
use crate::surface_control::Geometry;

/// Where `placement` puts a layer whose buffers are drawn `pixel_ratio` pixels to its unit, in its
/// parent's pixels; `None` for a layer nothing lets show: shrunk to nothing, clipped away or at no
/// opacity.
pub(super) fn of(placement: &Placement, pixel_ratio: f64) -> Option<Geometry> {
    let scale = placement.scale() / pixel_ratio;
    let visible = placement.visible_bounds();
    if placement.opacity() <= 0.0 || !(scale.is_finite() && scale > 0.0) || visible.is_empty() {
        return None;
    }
    let bounds = placement.bounds();
    let position = [bounds.left.round(), bounds.top.round()];
    let local = |parent: f64, origin: f64| (parent - origin) / scale;
    let crop = [
        local(visible.left, position[0]).floor(),
        local(visible.top, position[1]).floor(),
        local(visible.right, position[0]).ceil(),
        local(visible.bottom, position[1]).ceil(),
    ];
    Some(Geometry {
        position: position.map(|side| side as i32),
        scale: scale as f32,
        crop: crop.map(|side| side as i32),
    })
}

/// The box `placement` gives a layer whose buffers are drawn `pixel_ratio` pixels to its unit, in
/// the layer's own pixels.
pub(super) fn box_size(placement: &Placement, pixel_ratio: f64) -> [f64; 2] {
    [
        placement.rect.width() * pixel_ratio,
        placement.rect.height() * pixel_ratio,
    ]
}

/// Where a producer's buffer `size` pixels large lies in its layer: at its origin, unscaled, all of
/// it.
pub(super) fn unfitted(size: [u32; 2]) -> Geometry {
    Geometry {
        position: [0, 0],
        scale: 1.0,
        crop: [0, 0, size[0] as i32, size[1] as i32],
    }
}

/// Where a system's frame `size` pixels large, turned by `transform`, lies in a layer whose box is
/// `box_size` of its pixels: all of it, as large as the box holds, centred.
pub(super) fn fitted(size: [u32; 2], transform: BufferTransform, box_size: [f64; 2]) -> Geometry {
    let [width, height] = if transform.swaps_sides() {
        [size[1], size[0]]
    } else {
        size
    };
    let (width, height) = (f64::from(width.max(1)), f64::from(height.max(1)));
    let scale = (box_size[0] / width).min(box_size[1] / height);
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    Geometry {
        position: [
            ((box_size[0] - width * scale) / 2.0).round() as i32,
            ((box_size[1] - height * scale) / 2.0).round() as i32,
        ],
        scale: scale as f32,
        crop: [0, 0, width as i32, height as i32],
    }
}

/// Where a layer that fills a window `size` pixels large lies in it: at its origin, unscaled,
/// cropped to the window.
pub(super) fn filling(size: [u32; 2]) -> Geometry {
    Geometry {
        position: [0, 0],
        scale: 1.0,
        crop: [0, 0, size[0] as i32, size[1] as i32],
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Matrix4, Mutator, Rect};

    use super::*;

    /// A video 320 by 180 logical pixels at (20, 40) in a view of 2.5 pixels to the logical pixel:
    /// its buffer, drawn at 2.5, shows unscaled where the view's pixels put it.
    #[test]
    fn a_layer_drawn_at_the_views_ratio_shows_unscaled_where_its_box_lies() {
        let placement = Placement {
            rect: Rect::from_ltwh(20.0, 40.0, 320.0, 180.0),
            mutators: vec![Mutator::Transform(Matrix4::scale(2.5, 2.5))],
        };
        let geometry = of(&placement, 2.5).expect("it shows");
        assert_eq!(geometry.position, [50, 100]);
        assert_eq!(geometry.scale, 1.0);
        assert_eq!(geometry.crop, [0, 0, 800, 450]);
    }

    /// A card at half its size, a square of it clipped in the middle: the crop is the square in
    /// the layer's own pixels, twice its size in the view's.
    #[test]
    fn a_clip_crops_the_layer_in_its_own_pixels() {
        let placement = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 400.0, 400.0),
            mutators: vec![
                Mutator::Transform(Matrix4::scale(2.0, 2.0)),
                Mutator::ClipRect(Rect::from_ltwh(50.0, 50.0, 100.0, 100.0)),
                Mutator::Transform(Matrix4::scale(0.5, 0.5)),
            ],
        };
        let geometry = of(&placement, 2.0).expect("it shows");
        assert_eq!(geometry.position, [0, 0]);
        assert_eq!(geometry.scale, 0.5);
        assert_eq!(geometry.crop, [200, 200, 600, 600]);
    }

    #[test]
    fn a_layer_nothing_lets_show_is_not_placed() {
        let faded = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            mutators: vec![Mutator::Opacity(0)],
        };
        assert_eq!(of(&faded, 1.0), None);
        let clipped_away = Placement {
            rect: Rect::from_ltwh(200.0, 0.0, 10.0, 10.0),
            mutators: vec![Mutator::ClipRect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0))],
        };
        assert_eq!(of(&clipped_away, 1.0), None);
        let shrunk = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            mutators: vec![Mutator::Transform(Matrix4::scale(0.0, 0.0))],
        };
        assert_eq!(of(&shrunk, 1.0), None);
    }

    /// A camera's 640 by 480 frame turned a quarter, in a box 300 by 600: 480 by 640 upright,
    /// scaled to the box's width, centred down it.
    #[test]
    fn a_turned_frame_is_shown_whole_and_centred_in_the_box() {
        let geometry = fitted(
            [640, 480],
            BufferTransform::turned(90, false),
            [300.0, 600.0],
        );
        assert_eq!(geometry.scale, 0.625);
        assert_eq!(geometry.position, [0, 100]);
        assert_eq!(geometry.crop, [0, 0, 480, 640]);
        let unturned = fitted([640, 480], BufferTransform::IDENTITY, [640.0, 960.0]);
        assert_eq!(unturned.scale, 1.0);
        assert_eq!(unturned.position, [0, 240]);
    }

    #[test]
    fn a_placements_box_is_its_rect_in_the_layers_pixels() {
        let placement = Placement {
            rect: Rect::from_ltwh(20.0, 40.0, 320.0, 180.0),
            mutators: vec![Mutator::Transform(Matrix4::scale(2.5, 2.5))],
        };
        assert_eq!(box_size(&placement, 2.5), [800.0, 450.0]);
    }

    #[test]
    fn a_layer_that_fills_a_window_is_cropped_to_it() {
        assert_eq!(
            filling([1080, 2400]),
            Geometry {
                position: [0, 0],
                scale: 1.0,
                crop: [0, 0, 1080, 2400],
            }
        );
    }
}

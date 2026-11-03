//! A frame's layer tree turned into what a host composites, in paint order: the drawing between
//! composition layers, its slices, and each composition layer with what the containers above it
//! do to it.
//!
//! Flutter's rasterizer walks its tree with a stack of the containers open and, at each platform
//! view, has its embedder hand it a new canvas (`PlatformViewLayer::Paint`): the stack restores the
//! old canvas to where it started, so what was painted is a slice of its own, and reapplies every
//! open container on the new one (`LayerStateStack::set_delegate`). This walk does the same, and
//! hands the host the slices and the placements as data instead of calling it at each composition
//! layer, since a host here gets the tree itself. A host that shows every composition layer beneath
//! one buffer draws [`Slices::picture_with_room`]; one that wants a buffer above a composition
//! layer, as Flutter's iOS overlays are, draws each slice into its own, or walks the tree itself.
//!
//! A video's frames are drawn into the slice they lie in, under the containers open there, as
//! flow's `TextureLayer` paints from the registry its context carries: the host hands the walk
//! the image of each reader's frame as it draws, so the scene itself never holds a frame.
//!
//! The invariant a later editor must keep: a slice is a whole recording, every container open at
//! its ends closed in it, so the slices drawn in order draw what the tree paints; and a slice is
//! kept only when it paints, so a scene of composition layers alone has none.

use std::fmt;
use std::sync::Arc;

use super::Scene;
use super::layer::{ContainerLayer, DisplayListLayer, Effect, EmbeddedLayer, Layer};
use super::placement::{Mutator, Placement};
use crate::composition::CompositionLayer;
use crate::geometry::{Clip, Color, Matrix4, Offset, Rect, rrect_radii_elliptical};
use crate::media::VideoFrameReader;
use crate::painting::{
    Backdrop, BlendMode, Canvas, ClipOp, FillRule, Image, ImageFilter, Paint, Picture, valo,
};

/// The scene's slices and composition layers, in paint order: what Flutter's embedder hands its
/// compositor as `FlutterLayer`s, before any slice is rendered. `frame_image` answers the image
/// of the frame each video's reader shows in this draw, `None` for one with nothing to show; a
/// host without video answers `None` for all.
pub fn slices(
    scene: &Scene,
    frame_image: impl Fn(&Arc<dyn VideoFrameReader>) -> Option<Image>,
) -> Slices {
    let mut slicer = Slicer::new(&frame_image);
    slicer.visit_container(&scene.root);
    slicer.finish()
}

/// What [`slices`] makes of a scene.
#[derive(Debug)]
pub struct Slices {
    parts: Vec<Part>,
}

/// One part of a scene, in paint order.
pub enum Part {
    /// What the scene paints between two composition layers, or before the first or after the
    /// last, every container open at its ends closed in it: flow's `EmbedderViewSlice`. A stretch
    /// that paints nothing has none, as Flutter's embedder renders no slice without contents.
    Slice(Arc<Picture>),
    /// A composition layer, placed under the transforms, clips and opacities above it.
    CompositionLayer {
        layer: Arc<dyn CompositionLayer>,
        placement: Placement,
    },
}

impl fmt::Debug for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Part::Slice(picture) => f.debug_tuple("Slice").field(picture).finish(),
            Part::CompositionLayer { layer, placement } => f
                .debug_struct("CompositionLayer")
                .field("layer", &Arc::as_ptr(layer).cast::<()>())
                .field("placement", placement)
                .finish(),
        }
    }
}

impl Slices {
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    /// The composition layers the scene places, each with its placement, the bottom-most first.
    pub fn composition_layers(
        &self,
    ) -> impl Iterator<Item = (&Arc<dyn CompositionLayer>, &Placement)> {
        self.parts.iter().filter_map(|part| match part {
            Part::CompositionLayer { layer, placement } => Some((layer, placement)),
            Part::Slice(_) => None,
        })
    }

    /// Whether the scene paints anything besides its composition layers. One that does not changes
    /// no pixel of a buffer the composition layers show beneath.
    pub fn paints(&self) -> bool {
        self.parts.iter().any(|part| matches!(part, Part::Slice(_)))
    }

    /// The slices as one picture with room made at each composition layer between them: what a host
    /// draws that shows every composition layer beneath its one buffer. The room is the composition
    /// layer's rectangle cleared under the transforms and clips above it, by the opacity above it,
    /// so the buffer keeps that share of what was painted beneath and the composition layer shows
    /// through the rest. It is cleared between slices, in no layer of theirs, so it reaches the
    /// buffer.
    pub fn picture_with_room(&self) -> Picture {
        let mut canvas = Canvas::new();
        for part in &self.parts {
            match part {
                Part::Slice(picture) => canvas.draw_display_list(picture),
                Part::CompositionLayer { placement, .. } => make_room(&mut canvas, placement),
            }
        }
        canvas.build()
    }
}

/// The walk: the parts so far, the slice being recorded, the containers open, and where the
/// frames of videos come from.
struct Slicer<'a> {
    parts: Vec<Part>,
    slice: SliceRecording,
    /// The effects of the containers the walk is in, the root's first.
    open: Vec<&'a Effect>,
    frame_image: &'a dyn Fn(&Arc<dyn VideoFrameReader>) -> Option<Image>,
}

impl<'a> Slicer<'a> {
    fn new(frame_image: &'a dyn Fn(&Arc<dyn VideoFrameReader>) -> Option<Image>) -> Slicer<'a> {
        Slicer {
            parts: Vec::new(),
            slice: SliceRecording::new(),
            open: Vec::new(),
            frame_image,
        }
    }

    fn visit(&mut self, layer: &'a Layer) {
        match layer {
            Layer::Container(container) => self.visit_container(container),
            Layer::DisplayList(picture) => self.slice.draw_picture(picture),
            Layer::Embedded(embedded) => self.place_composition_layer(embedded),
            Layer::Video(video) => {
                if let Some(image) = (self.frame_image)(&video.reader) {
                    self.slice.draw_image(&image, video.rect);
                }
            }
        }
    }

    fn visit_container(&mut self, container: &'a ContainerLayer) {
        self.slice.open(&container.effect);
        self.open.push(&container.effect);
        for child in &container.children {
            self.visit(child);
        }
        self.open.pop();
        self.slice.close();
    }

    /// What was painted so far becomes a slice of its own, and the composition layer is placed
    /// above it with what the open containers do.
    fn place_composition_layer(&mut self, embedded: &EmbeddedLayer) {
        self.end_slice();
        self.parts.push(Part::CompositionLayer {
            layer: Arc::clone(&embedded.layer),
            placement: Placement {
                rect: embedded.rect,
                mutators: mutators_of(&self.open),
            },
        });
    }

    /// Closes the open containers on the slice being recorded, keeps it if it paints, and goes on
    /// in a new one with the same containers open.
    fn end_slice(&mut self) {
        let next = SliceRecording::reopening(&self.open);
        let ended = std::mem::replace(&mut self.slice, next);
        self.keep(ended.finish(self.open.len()));
    }

    fn keep(&mut self, slice: Option<Arc<Picture>>) {
        self.parts.extend(slice.map(Part::Slice));
    }

    fn finish(mut self) -> Slices {
        let last = std::mem::replace(&mut self.slice, SliceRecording::new());
        self.keep(last.finish(0));
        Slices { parts: self.parts }
    }
}

/// One slice as it is recorded: the canvas, with a scope open for each container the walk is
/// in, and whether anything drawn in it changes a pixel.
struct SliceRecording {
    canvas: Canvas,
    paints: bool,
}

impl SliceRecording {
    fn new() -> SliceRecording {
        SliceRecording {
            canvas: Canvas::new(),
            paints: false,
        }
    }

    /// A slice that goes on under the containers `open`, the root's first.
    fn reopening(open: &[&Effect]) -> SliceRecording {
        let mut slice = SliceRecording::new();
        for effect in open {
            slice.reopen(effect);
        }
        slice
    }

    /// Opens a scope doing what `effect` does, as the walk enters its container.
    fn open(&mut self, effect: &Effect) {
        if let Effect::BackdropFilter {
            filter,
            blend_mode,
            backdrop_id,
        } = effect
        {
            let backdrop = valo_backdrop(filter, *backdrop_id);
            self.canvas
                .save_layer_backdrop(None, &blending(*blend_mode), backdrop);
            self.paints = true;
        } else {
            self.reopen(effect);
        }
    }

    /// Opens a scope doing what `effect` does in a slice after the one its container began in. A
    /// backdrop filter has filtered what lies beneath already, so it opens a plain layer of its
    /// blend mode, as flow's `BackdropFilterEntry::reapply` does.
    fn reopen(&mut self, effect: &Effect) {
        let canvas = &mut self.canvas;
        match effect {
            Effect::None => canvas.save(),
            Effect::Transform(matrix) => {
                canvas.save();
                canvas.concat(matrix);
            }
            Effect::ClipRect {
                rect,
                clip_behavior,
            } => {
                open_clip(canvas, *clip_behavior, (*rect).into());
                canvas.clip_rect(*rect, ClipOp::Intersect);
            }
            Effect::ClipRRect {
                rrect,
                clip_behavior,
            } => {
                open_clip(canvas, *clip_behavior, rrect.outer_rect().into());
                canvas.clip_rrect_radii_elliptical(
                    rrect.outer_rect(),
                    rrect_radii_elliptical(*rrect),
                    ClipOp::Intersect,
                );
            }
            Effect::ClipPath {
                path,
                clip_behavior,
            } => {
                open_clip(canvas, *clip_behavior, path.bounds());
                canvas.clip_path(path, FillRule::NonZero, ClipOp::Intersect);
            }
            Effect::Opacity { alpha, offset } => {
                let paint = Paint {
                    color: Color::from_argb(i32::from(*alpha), 0, 0, 0).into(),
                    ..Paint::default()
                };
                canvas.save_layer(None, &paint);
                canvas.translate(offset.dx() as f32, offset.dy() as f32);
            }
            Effect::BackdropFilter { blend_mode, .. } => {
                canvas.save_layer(None, &blending(*blend_mode));
            }
        }
    }

    /// Closes the innermost scope.
    fn close(&mut self) {
        self.canvas.restore();
    }

    fn draw_picture(&mut self, layer: &DisplayListLayer) {
        let canvas = &mut self.canvas;
        canvas.save();
        canvas.translate(layer.offset.dx() as f32, layer.offset.dy() as f32);
        canvas.draw_display_list(&layer.picture);
        canvas.restore();
        self.paints |= layer.picture.draw_count() > 0;
    }

    /// The whole of `image` drawn into `rect`, sampled linearly.
    fn draw_image(&mut self, image: &Image, rect: Rect) {
        self.canvas
            .draw_image(image, rect.into(), &Paint::default());
        self.paints = true;
    }

    /// The slice, its `open` scopes closed; `None` when it paints nothing.
    fn finish(mut self, open: usize) -> Option<Arc<Picture>> {
        for _ in 0..open {
            self.close();
        }
        self.paints.then(|| Arc::new(self.canvas.build()))
    }
}

/// What the containers whose effects are `open` do to a composition layer placed in them, the
/// outermost first. An offset opacity moves before it fades, as flow's `OpacityLayer` translates
/// before it applies its opacity. A backdrop filter does nothing to a composition layer beneath the
/// buffer it reads.
fn mutators_of(open: &[&Effect]) -> Vec<Mutator> {
    let mut mutators = Vec::new();
    for effect in open {
        match effect {
            Effect::None | Effect::BackdropFilter { .. } => {}
            Effect::Transform(matrix) => mutators.push(Mutator::Transform(*matrix)),
            Effect::ClipRect { rect, .. } => mutators.push(Mutator::ClipRect(*rect)),
            Effect::ClipRRect { rrect, .. } => mutators.push(Mutator::ClipRRect(*rrect)),
            Effect::ClipPath { path, .. } => mutators.push(Mutator::ClipPath(Arc::clone(path))),
            Effect::Opacity { alpha, offset } => {
                if *offset != Offset::ZERO {
                    mutators.push(Mutator::Transform(translation(*offset)));
                }
                mutators.push(Mutator::Opacity(*alpha));
            }
        }
    }
    mutators
}

/// Clears `placement`'s rectangle under the transforms and clips above it, by the opacity above
/// it. A composition layer nothing lets show gets no room.
fn make_room(canvas: &mut Canvas, placement: &Placement) {
    let opacity = placement.opacity();
    if opacity <= 0.0 {
        return;
    }
    canvas.save();
    for mutator in &placement.mutators {
        apply(canvas, mutator);
    }
    canvas.draw_rect(placement.rect, &clearing(opacity));
    canvas.restore();
}

/// Records what `mutator` does to what is drawn after it; an opacity is the caller's to apply.
fn apply(canvas: &mut Canvas, mutator: &Mutator) {
    match mutator {
        Mutator::Transform(matrix) => canvas.concat(matrix),
        Mutator::ClipRect(rect) => canvas.clip_rect(*rect, ClipOp::Intersect),
        Mutator::ClipRRect(rrect) => canvas.clip_rrect_radii_elliptical(
            rrect.outer_rect(),
            rrect_radii_elliptical(*rrect),
            ClipOp::Intersect,
        ),
        Mutator::ClipPath(path) => canvas.clip_path(path, FillRule::NonZero, ClipOp::Intersect),
        Mutator::Opacity(_) => {}
    }
}

/// The paint that takes `opacity` of what lies beneath away: all of it at 1, a clear hole.
fn clearing(opacity: f64) -> Paint {
    Paint {
        color: valo::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: opacity as f32,
        },
        blend_mode: BlendMode::DstOut,
        ..Paint::default()
    }
}

/// The save that starts a clip scope. `Clip::AntiAliasWithSaveLayer` opens a layer over the
/// clip's bounds, as the engine's clip layers do.
fn open_clip(canvas: &mut Canvas, clip_behavior: Clip, bounds: valo::Rect) {
    match clip_behavior {
        Clip::None | Clip::HardEdge | Clip::AntiAlias => canvas.save(),
        Clip::AntiAliasWithSaveLayer => canvas.save_layer(Some(bounds), &Paint::default()),
    }
}

fn blending(blend_mode: BlendMode) -> Paint {
    Paint {
        blend_mode,
        ..Paint::default()
    }
}

fn translation(offset: Offset) -> Matrix4 {
    Matrix4::translation(offset.dx() as f32, offset.dy() as f32)
}

/// The valo [`Backdrop`] that samples the scene through `filter`, shared by every backdrop with
/// the same `backdrop_id`.
fn valo_backdrop(filter: &ImageFilter, backdrop_id: Option<u64>) -> Backdrop {
    let backdrop = Backdrop::new(filter.to_valo());
    match backdrop_id {
        Some(id) => backdrop.shared(id),
        None => backdrop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SceneBuilder;
    use crate::geometry::{RRect, Radius, Rect};
    use crate::painting::{ColorFilter, flattened_ops};

    fn square() -> Arc<Picture> {
        let mut canvas = Canvas::new();
        canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), &Paint::default());
        Arc::new(canvas.build())
    }

    fn idle_layer() -> Arc<dyn CompositionLayer> {
        super::super::idle_layer()
    }

    #[test]
    fn a_composition_layer_carries_the_transforms_clips_and_opacity_above_it() {
        let layer = idle_layer();
        let rrect = RRect::from_rect_and_radius(
            Rect::from_ltwh(0.0, 0.0, 50.0, 50.0),
            Radius::circular(8.0),
        );
        let mut builder = SceneBuilder::new();
        builder.push_transform(&Matrix4::scale(2.0, 2.0));
        builder.push_clip_rect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0), Clip::HardEdge);
        builder.push_opacity(128, Offset::new(5.0, 6.0));
        builder.push_backdrop_filter(&ImageFilter::blur(2.0, 2.0), BlendMode::SrcOver, None);
        builder.push_clip_rrect(rrect, Clip::AntiAlias);
        builder.add_embedded_layer(Arc::clone(&layer), Rect::from_ltwh(1.0, 2.0, 30.0, 20.0));
        for _ in 0..5 {
            builder.pop();
        }
        let slices = slices(&builder.build(), |_| None);
        let [(placed, placement)] = slices.composition_layers().collect::<Vec<_>>()[..] else {
            panic!("one layer: {:?}", slices.parts());
        };
        assert!(Arc::ptr_eq(placed, &layer));
        assert_eq!(
            placement,
            &Placement {
                rect: Rect::from_ltwh(1.0, 2.0, 30.0, 20.0),
                mutators: vec![
                    Mutator::Transform(Matrix4::scale(2.0, 2.0)),
                    Mutator::ClipRect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0)),
                    Mutator::Transform(Matrix4::translation(5.0, 6.0)),
                    Mutator::Opacity(128),
                    Mutator::ClipRRect(rrect),
                ],
            },
            "the backdrop filter does nothing to the layer"
        );
    }

    #[test]
    fn a_composition_layer_in_a_retained_container_is_placed_again() {
        let mut builder = SceneBuilder::new();
        builder.push_offset(10.0, 0.0);
        builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(0.0, 0.0, 4.0, 4.0));
        let retained = builder.pop();
        let _first = builder.build();

        let mut builder = SceneBuilder::new();
        builder.push_offset(0.0, 20.0);
        builder.add_retained(&retained);
        builder.pop();
        let slices = slices(&builder.build(), |_| None);
        let (_, placement) = slices.composition_layers().next().unwrap();
        assert_eq!(placement.bounds(), Rect::from_ltwh(10.0, 20.0, 4.0, 4.0));
    }

    #[test]
    fn a_composition_layer_ends_the_slice_and_the_next_opens_under_the_same_containers() {
        let mut builder = SceneBuilder::new();
        builder.push_opacity(128, Offset::ZERO);
        builder.add_picture(Offset::ZERO, &square(), false, false);
        builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(0.0, 0.0, 4.0, 4.0));
        builder.add_picture(Offset::new(20.0, 0.0), &square(), false, false);
        builder.pop();
        let slices = slices(&builder.build(), |_| None);
        let [
            Part::Slice(beneath),
            Part::CompositionLayer { .. },
            Part::Slice(above),
        ] = slices.parts()
        else {
            panic!(
                "a slice, the composition layer, a slice: {:?}",
                slices.parts()
            );
        };
        for slice in [beneath, above] {
            let layers = slice
                .ops()
                .iter()
                .filter(|op| matches!(op, valo::Op::SaveLayer { .. }))
                .count();
            let restores = slice
                .ops()
                .iter()
                .filter(|op| matches!(op, valo::Op::Restore))
                .count();
            assert_eq!(layers, 1, "the opacity in each slice: {:?}", slice.ops());
            assert_eq!(restores, 3, "the root, the opacity and the picture's own");
        }
        assert!(slices.paints());
    }

    #[test]
    fn a_scene_of_composition_layers_alone_paints_nothing() {
        let mut builder = SceneBuilder::new();
        builder.push_opacity(128, Offset::ZERO);
        builder.push_clip_rect(
            Rect::from_ltwh(0.0, 0.0, 9.0, 9.0),
            Clip::AntiAliasWithSaveLayer,
        );
        builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(0.0, 0.0, 4.0, 4.0));
        builder.add_picture(Offset::ZERO, &Arc::new(Canvas::new().build()), false, false);
        builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(4.0, 0.0, 4.0, 4.0));
        builder.pop();
        builder.pop();
        let slices = slices(&builder.build(), |_| None);
        assert!(
            slices
                .parts()
                .iter()
                .all(|part| matches!(part, Part::CompositionLayer { .. })),
            "{:?}",
            slices.parts()
        );
        assert!(!slices.paints());
        assert_eq!(slices.composition_layers().count(), 2);
    }

    #[test]
    fn a_backdrop_filter_paints_though_nothing_is_drawn_in_it() {
        let mut builder = SceneBuilder::new();
        builder.push_backdrop_filter(&ImageFilter::blur(2.0, 2.0), BlendMode::SrcOver, None);
        builder.pop();
        assert!(slices(&builder.build(), |_| None).paints());
    }

    #[test]
    fn room_is_cleared_under_the_composition_layers_transforms_and_clips_between_the_slices() {
        let mut builder = SceneBuilder::new();
        builder.add_picture(Offset::ZERO, &square(), false, false);
        builder.push_transform(&Matrix4::scale(2.0, 2.0));
        builder.push_clip_rect(Rect::from_ltwh(0.0, 0.0, 3.0, 3.0), Clip::HardEdge);
        builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(1.0, 1.0, 4.0, 4.0));
        builder.pop();
        builder.pop();
        let picture = slices(&builder.build(), |_| None).picture_with_room();
        let ops = picture.ops();
        assert!(
            matches!(ops[0], valo::Op::DrawDisplayList { .. }),
            "the slice first: {ops:?}"
        );
        let hole = ops
            .iter()
            .position(|op| matches!(op, valo::Op::DrawRect { paint, .. } if paint.blend_mode == BlendMode::DstOut))
            .expect("a hole");
        let before_hole = &ops[1..hole];
        assert!(
            before_hole
                .iter()
                .any(|op| matches!(op, valo::Op::Transform(_)))
                && before_hole
                    .iter()
                    .any(|op| matches!(op, valo::Op::ClipPath { .. })),
            "{ops:?}"
        );
        assert!(
            !ops.iter()
                .any(|op| matches!(op, valo::Op::SaveLayer { .. })),
            "the hole is in no layer: {ops:?}"
        );
    }

    #[test]
    fn a_composition_layer_nothing_lets_show_gets_no_room() {
        let mut builder = SceneBuilder::new();
        builder.add_picture(Offset::ZERO, &square(), false, false);
        builder.push_opacity(0, Offset::ZERO);
        builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(1.0, 1.0, 4.0, 4.0));
        builder.pop();
        let picture = slices(&builder.build(), |_| None).picture_with_room();
        assert!(
            !flattened_ops(&picture)
                .iter()
                .any(|op| matches!(op, valo::Op::DrawRect { paint, .. } if paint.blend_mode == BlendMode::DstOut)),
        );
    }

    #[test]
    fn a_colour_filter_composed_over_a_blur_reaches_the_backdrop_in_its_order() {
        let saturation = ColorFilter::saturation(1.8);
        let filter =
            ImageFilter::compose(ImageFilter::Color(saturation), ImageFilter::blur(4.0, 4.0));
        let backdrop = recorded_backdrop(&filter, Some(7));
        assert_eq!(
            backdrop.filter,
            Some(valo::ImageFilter::compose(
                valo::ImageFilter::Color(saturation.to_valo()),
                valo::ImageFilter::blur(4.0, 4.0),
            ))
        );
        assert_eq!(backdrop.key, Some(7), "the backdrop is still shared");
    }

    #[test]
    fn an_uneven_blur_reaches_the_backdrop_with_both_sigmas() {
        let backdrop = recorded_backdrop(&ImageFilter::blur(2.0, 6.0), None);
        assert_eq!(backdrop.filter, Some(valo::ImageFilter::blur(2.0, 6.0)));
        assert_eq!(backdrop.key, None);
    }

    #[test]
    fn a_bounded_blur_reaches_the_backdrop_with_its_bounds_and_leaves_the_layer_unconfined() {
        let glass = Rect::from_ltwh(8.0, 8.0, 16.0, 16.0);
        let bounded = recorded_backdrop(&ImageFilter::blur(2.0, 3.0).bounds(glass), None);
        assert_eq!(
            bounded.filter,
            Some(valo::ImageFilter::bounded_blur(2.0, 3.0, glass.into()))
        );
        let unbounded = recorded_backdrop(&ImageFilter::blur(2.0, 3.0), None);
        assert_eq!(
            bounded.bounds, unbounded.bounds,
            "the bounds limit what the blur reads, not the layer"
        );
    }

    /// A backdrop filter over a composition layer opens its backdrop in the slice beneath the
    /// composition layer only: above it, it is a plain layer, as flow's reapplied backdrop entry
    /// is.
    #[test]
    fn a_backdrop_filter_reads_the_backdrop_once_across_a_composition_layer() {
        let mut builder = SceneBuilder::new();
        builder.push_backdrop_filter(&ImageFilter::blur(2.0, 2.0), BlendMode::SrcOver, None);
        builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(0.0, 0.0, 4.0, 4.0));
        builder.add_picture(Offset::ZERO, &square(), false, false);
        builder.pop();
        let picture = slices(&builder.build(), |_| None).picture_with_room();
        assert_eq!(picture.backdrop_reads(), 1);
        let layers = flattened_ops(&picture)
            .into_iter()
            .filter(|op| matches!(op, valo::Op::SaveLayer { .. }))
            .count();
        assert_eq!(
            layers, 2,
            "the backdrop's, and the plain one above the composition layer"
        );
    }

    /// What a scene records of the one backdrop a push opened.
    struct RecordedBackdrop {
        filter: Option<valo::ImageFilter>,
        /// What the backdrop's snapshot is shared under.
        key: Option<u64>,
        /// The bounds of the backdrop's layer.
        bounds: valo::Bounds,
    }

    fn recorded_backdrop(filter: &ImageFilter, backdrop_id: Option<u64>) -> RecordedBackdrop {
        let mut builder = SceneBuilder::new();
        builder.push_backdrop_filter(filter, BlendMode::SrcOver, backdrop_id);
        builder.pop();
        flattened_ops(&slices(&builder.build(), |_| None).picture_with_room())
            .into_iter()
            .find_map(|op| match op {
                valo::Op::SaveLayer {
                    backdrop,
                    scope_bounds,
                    ..
                } => Some(RecordedBackdrop {
                    key: backdrop.as_ref().and_then(|backdrop| backdrop.shared_key),
                    filter: backdrop.map(|backdrop| backdrop.filter),
                    bounds: scope_bounds,
                }),
                _ => None,
            })
            .expect("a backdrop filter opens a layer")
    }

    /// Checks of what a scene shows once drawn. They need `test_support`'s GPU device, so they
    /// run where the `test-support` feature is on, as it is in a test run of the workspace.
    /// A reader with no frame of its own: what a video shows before its first frame.
    struct NoFrames;

    impl VideoFrameReader for NoFrames {
        fn frame_for(
            &self,
            _display_time: std::time::Instant,
        ) -> Option<Arc<dyn crate::VideoFrame>> {
            None
        }
    }

    /// The walk asks the host for each video's frame as it draws it, which is how a host knows
    /// the readers it shows; one with nothing to show draws nothing.
    #[test]
    fn a_video_asks_for_its_frame_and_draws_nothing_without_one() {
        let reader: Arc<dyn VideoFrameReader> = Arc::new(NoFrames);
        let mut builder = SceneBuilder::new();
        builder.add_video(Arc::clone(&reader), Rect::from_ltwh(0.0, 0.0, 4.0, 4.0));
        let asked = std::cell::Cell::new(0);
        let slices = slices(&builder.build(), |asked_for| {
            assert!(Arc::ptr_eq(asked_for, &reader));
            asked.set(asked.get() + 1);
            None
        });
        assert_eq!(asked.get(), 1);
        assert!(!slices.paints(), "{:?}", slices.parts());
    }

    #[cfg(feature = "test-support")]
    mod drawn {
        use super::*;
        use crate::test_support::drawn_pixels;

        const SIZE: u32 = 32;

        /// A video's frame is part of the picture: drawn into its rectangle, cut by the clip
        /// above it like anything painted there.
        #[test]
        fn a_video_frame_is_drawn_in_its_slice_under_the_containers_above_it() {
            let reader: Arc<dyn VideoFrameReader> = Arc::new(NoFrames);
            let whole = Rect::from_ltwh(0.0, 0.0, SIZE as f64, SIZE as f64);
            let mut builder = SceneBuilder::new();
            builder.push_clip_rect(
                Rect::from_ltwh(0.0, 0.0, SIZE as f64 / 2.0, SIZE as f64),
                Clip::HardEdge,
            );
            builder.add_video(reader, whole);
            builder.pop();
            let frame = crate::test_support::solid_image([4, 4], [255, 0, 0, 255]);
            let slices = slices(&builder.build(), |_| Some(frame.clone()));
            assert!(slices.paints());
            let pixels = drawn_pixels(&slices.picture_with_room(), [SIZE, SIZE]);
            assert_near(
                pixel_at(&pixels, 8, 16),
                [255, 0, 0, 255],
                "inside the clip",
            );
            assert_eq!(pixel_at(&pixels, 24, 16)[3], 0, "outside the clip");
        }

        #[test]
        fn a_colour_filter_composed_over_a_blur_recolours_the_backdrop() {
            let swap_red_and_green = ColorFilter::matrix([
                0.0, 1.0, 0.0, 0.0, 0.0, //
                1.0, 0.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 1.0, 0.0, //
            ]);
            let glass = Rect::from_ltwh(8.0, 8.0, 16.0, 16.0);
            let filter = ImageFilter::compose(
                ImageFilter::Color(swap_red_and_green),
                ImageFilter::blur(2.0, 2.0).bounds(glass),
            );
            let mut builder = SceneBuilder::new();
            builder.add_picture(Offset::ZERO, &filled(Color::new(0xFFFF0000)), false, false);
            builder.push_clip_rect(glass, Clip::HardEdge);
            builder.push_backdrop_filter(&filter, BlendMode::SrcOver, None);
            builder.pop();
            builder.pop();
            let pixels = drawn(&builder.build());
            assert_near(
                pixel_at(&pixels, 16, 16),
                [0, 255, 0, 255],
                "inside the glass",
            );
            assert_near(
                pixel_at(&pixels, 2, 2),
                [255, 0, 0, 255],
                "outside the glass",
            );
        }

        /// A composition layer under glass shows through it, unfiltered: the room is made after the
        /// backdrop's layer closes, so it reaches the buffer.
        #[test]
        fn a_composition_layer_under_a_backdrop_filter_shows_through_it() {
            let mut builder = SceneBuilder::new();
            builder.add_picture(Offset::ZERO, &filled(Color::new(0xFFFFFFFF)), false, false);
            builder.push_backdrop_filter(&ImageFilter::blur(2.0, 2.0), BlendMode::SrcOver, None);
            builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(0.0, 0.0, 16.0, 32.0));
            builder.pop();
            let pixels = drawn(&builder.build());
            assert_eq!(pixel_at(&pixels, 8, 16)[3], 0, "over the composition layer");
            assert_near(pixel_at(&pixels, 28, 16), [255, 255, 255, 255], "beside it");
        }

        /// The hole keeps half of what was painted beneath a composition layer at half opacity, and
        /// what the same opacity paints above the composition layer stays.
        #[test]
        fn a_composition_layer_under_an_opacity_shows_through_by_that_opacity() {
            let mut builder = SceneBuilder::new();
            builder.add_picture(Offset::ZERO, &filled(Color::new(0xFFFFFFFF)), false, false);
            builder.push_opacity(128, Offset::ZERO);
            builder.add_embedded_layer(idle_layer(), Rect::from_ltwh(0.0, 0.0, 16.0, 32.0));
            builder.add_picture(
                Offset::ZERO,
                &square_of(Color::new(0xFF0000FF), 4.0),
                false,
                false,
            );
            builder.pop();
            let pixels = drawn(&builder.build());
            assert_near(
                pixel_at(&pixels, 24, 16),
                [255, 255, 255, 255],
                "beside the composition layer",
            );
            assert_near(
                pixel_at(&pixels, 8, 16),
                [255, 255, 255, 127],
                "over the composition layer",
            );
            let above = pixel_at(&pixels, 1, 1);
            assert!(
                above[2] > above[0] && above[3] > 127,
                "the blue square painted above the composition layer: {above:?}"
            );
        }

        #[test]
        fn a_hole_under_a_rounded_clip_keeps_the_corners() {
            let mut builder = SceneBuilder::new();
            builder.add_picture(Offset::ZERO, &filled(Color::new(0xFFFFFFFF)), false, false);
            builder.push_clip_rrect(
                RRect::from_rect_and_radius(
                    Rect::from_ltwh(0.0, 0.0, SIZE.into(), SIZE.into()),
                    Radius::circular(12.0),
                ),
                Clip::AntiAlias,
            );
            builder.add_embedded_layer(
                idle_layer(),
                Rect::from_ltwh(0.0, 0.0, SIZE.into(), SIZE.into()),
            );
            builder.pop();
            let pixels = drawn(&builder.build());
            assert_near(
                pixel_at(&pixels, 0, 0),
                [255, 255, 255, 255],
                "in the corner",
            );
            assert_eq!(pixel_at(&pixels, 16, 16)[3], 0, "in the middle");
        }

        fn drawn(scene: &Scene) -> Vec<u8> {
            drawn_pixels(&slices(scene, |_| None).picture_with_room(), [SIZE, SIZE])
        }

        /// A picture that covers the whole scene in `color`.
        fn filled(color: Color) -> Arc<Picture> {
            square_of(color, SIZE.into())
        }

        fn square_of(color: Color, side: f64) -> Arc<Picture> {
            let mut canvas = Canvas::new();
            let paint = Paint {
                color: color.into(),
                ..Paint::default()
            };
            canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, side, side), &paint);
            Arc::new(canvas.build())
        }

        fn pixel_at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
            let start = (y * SIZE as usize + x) * 4;
            pixels[start..start + 4]
                .try_into()
                .expect("four bytes a pixel")
        }

        /// Within a step or two of rounding in each channel.
        fn assert_near(actual: [u8; 4], expected: [u8; 4], place: &str) {
            let near = actual
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual.abs_diff(expected) <= 2);
            assert!(near, "{place}: {actual:?}, expected {expected:?}");
        }
    }
}

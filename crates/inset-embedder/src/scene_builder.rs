//! Flutter counterpart: dart:ui `SceneBuilder` / `Scene`.
//!
//! The engine's builder composites retained `EngineLayer`s; valo composites a display list, so
//! every push opens a nested recording that `pop` closes into a list of its own: that list is
//! the engine layer, embedded in the enclosing recording and handed back for the layer to keep,
//! and `add_retained` embeds it again while the subtree has not changed (see the rendering
//! crate's `PORTING.md`). A native surface the host composites is not in the list: the
//! builder keeps the transform and the clip of its open scopes so that `add_surface` can place
//! it in the view's coordinates, at the scale it is drawn and cut to what shows, and the scene
//! carries those placements beside the picture.

use std::fmt;
use std::sync::Arc;

use crate::composition::Surface;
use crate::geometry::{
    Clip, Color, Matrix4, Offset, RRect, Rect, rrect_radii_elliptical, transform3,
};
use crate::painting::{
    Backdrop, BlendMode, Canvas, ClipOp, FillRule, ImageFilter, Paint, Path, Picture, valo,
};

/// Flutter `Scene` — what [`SceneBuilder::build`] produces and [`crate::View::present`] takes:
/// the framework's picture, and the native surfaces this frame places, bottom-most first.
/// A host without a compositor uses the picture alone.
#[derive(Debug, PartialEq)]
pub struct Scene {
    pub picture: Arc<Picture>,
    pub surfaces: Vec<SurfacePlacement>,
}

/// Where one frame puts one native surface. The surface is its producer's own handle, so a
/// host reaches the native object behind it directly and moves it.
#[derive(Clone)]
pub struct SurfacePlacement {
    pub surface: Arc<dyn Surface>,
    /// In the view's physical pixels, with the transforms above the surface applied: the
    /// scene's own space, whose root transform scales logical pixels by the device pixel ratio.
    pub rect: Rect,
    /// How many of the view's physical pixels one unit of the surface's own coordinates spans:
    /// the scale of the transforms above it, the root's device pixel ratio among them. A
    /// producer draws its buffers at the device pixel ratio, so a host shows a buffer at
    /// `scale / device_pixel_ratio` of its size. Under a scale that differs between the axes,
    /// or a rotation, it is the scale along the surface's own horizontal.
    pub scale: f64,
    /// The part of `rect` that shows, in the same pixels: `rect` cut down to the bounds of
    /// every clip above the surface. A host shows nothing of the surface outside it. Where a
    /// clip is not a rectangle, what lies between it and its bounds is the picture's to cover.
    pub clip: Rect,
}

/// Two placements are equal when they put the same surface in the same place: what a host
/// compares to tell a frame worth presenting from one it already shows.
impl PartialEq for SurfacePlacement {
    fn eq(&self, other: &SurfacePlacement) -> bool {
        Arc::ptr_eq(&self.surface, &other.surface)
            && self.rect == other.rect
            && self.scale == other.scale
            && self.clip == other.clip
    }
}

impl fmt::Debug for SurfacePlacement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfacePlacement")
            .field("surface", &Arc::as_ptr(&self.surface).cast::<()>())
            .field("rect", &self.rect)
            .field("scale", &self.scale)
            .field("clip", &self.clip)
            .finish()
    }
}

/// Flutter `EngineLayer` — the display list a layer's subtree recorded when it was last added to
/// a scene, which [`SceneBuilder::add_retained`] embeds again while nothing in it changed. The
/// engine keeps a node of its layer tree; valo keeps a nested list.
pub type EngineLayer = Arc<Picture>;

/// Builds a [`Scene`] containing the given visuals.
///
/// A [`Scene`] can then be rendered using [`crate::View::present`].
///
/// To draw graphical operations onto a [`Scene`], first create a [`Canvas`], record into it, and
/// add the resulting [`Picture`] with [`add_picture`](Self::add_picture). Each `push_*` opens a
/// compositing scope that [`pop`](Self::pop) closes into an [`EngineLayer`]; unbalanced scopes
/// are a bug the builder asserts on.
pub struct SceneBuilder {
    /// The recording of the innermost open scope, or of the scene itself outside any.
    canvas: Canvas,
    /// What the open scope's coordinates are in the view's: the transforms pushed so far.
    transform: Matrix4,
    /// The bounds of the clips pushed so far, in the view's coordinates; `None` while nothing
    /// clips.
    clip: Option<Rect>,
    /// The scopes enclosing the open one, outermost first; every `push_*` adds one and every
    /// `pop` takes one back. dart:ui asserts the same balance.
    enclosing: Vec<EnclosingScope>,
    /// The native surfaces placed so far, in paint order.
    surfaces: Vec<SurfacePlacement>,
}

/// A scope as it was when a `push_*` opened the next one inside it.
struct EnclosingScope {
    canvas: Canvas,
    transform: Matrix4,
    clip: Option<Rect>,
}

impl fmt::Debug for SceneBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SceneBuilder")
            .field("open_scopes", &self.enclosing.len())
            .finish_non_exhaustive()
    }
}

impl Default for SceneBuilder {
    fn default() -> SceneBuilder {
        SceneBuilder::new()
    }
}

impl SceneBuilder {
    /// Creates an empty [`SceneBuilder`] object.
    pub fn new() -> SceneBuilder {
        SceneBuilder {
            canvas: Canvas::new(),
            transform: Matrix4::IDENTITY,
            clip: None,
            enclosing: Vec::new(),
            surfaces: Vec::new(),
        }
    }

    /// Pushes a transform operation onto the operation stack.
    ///
    /// The objects are transformed by the given matrix before rasterization.
    pub fn push_transform(&mut self, matrix4: &Matrix4) {
        self.open(|canvas| {
            canvas.save();
            canvas.concat(matrix4);
        });
        self.transformed_by(matrix4);
    }

    /// Pushes an offset operation onto the operation stack.
    ///
    /// This is equivalent to [`push_transform`](Self::push_transform) with a matrix with only
    /// translation.
    pub fn push_offset(&mut self, dx: f64, dy: f64) {
        self.open(|canvas| {
            canvas.save();
            canvas.translate(dx as f32, dy as f32);
        });
        self.transformed_by(&Matrix4::translation(dx as f32, dy as f32));
    }

    /// Pushes a rectangular clip operation onto the operation stack.
    ///
    /// Rasterization outside the given rectangle is discarded.
    pub fn push_clip_rect(&mut self, rect: Rect, clip_behavior: Clip) {
        self.open(|canvas| {
            open_clip(canvas, clip_behavior, rect.into());
            canvas.clip_rect(rect, ClipOp::Intersect);
        });
        self.clipped_to(rect);
    }

    /// Pushes a rounded-rectangular clip operation onto the operation stack.
    ///
    /// Rasterization outside the given rounded rectangle is discarded.
    pub fn push_clip_rrect(&mut self, rrect: RRect, clip_behavior: Clip) {
        self.open(|canvas| {
            open_clip(canvas, clip_behavior, rrect.outer_rect().into());
            canvas.clip_rrect_radii_elliptical(
                rrect.outer_rect(),
                rrect_radii_elliptical(rrect),
                ClipOp::Intersect,
            );
        });
        self.clipped_to(rrect.outer_rect());
    }

    /// Pushes a path clip operation onto the operation stack.
    ///
    /// Rasterization outside the given path is discarded.
    pub fn push_clip_path(&mut self, path: &Arc<Path>, clip_behavior: Clip) {
        self.open(|canvas| {
            open_clip(canvas, clip_behavior, path.bounds());
            canvas.clip_path(path, FillRule::NonZero, ClipOp::Intersect);
        });
        self.clipped_to(bounds_of(path));
    }

    /// Pushes an opacity operation onto the operation stack.
    ///
    /// The given alpha value is blended into the alpha value of the objects' rasterization. An
    /// alpha value of 0 makes the objects entirely invisible. An alpha value of 255 has no
    /// effect (i.e., the objects retain the current opacity).
    pub fn push_opacity(&mut self, alpha: i32, offset: Offset) {
        self.open(|canvas| {
            let paint = Paint {
                color: Color::from_argb(alpha, 0, 0, 0).into(),
                ..Paint::default()
            };
            canvas.save_layer(None, &paint);
            canvas.translate(offset.dx() as f32, offset.dy() as f32);
        });
        self.transformed_by(&Matrix4::translation(
            offset.dx() as f32,
            offset.dy() as f32,
        ));
    }

    /// Pushes a backdrop filter operation onto the operation stack.
    ///
    /// The given filter is applied to the current contents of the scene as far back as the most
    /// recent save layer and rendered back to the scene using the indicated `blend_mode` prior to
    /// rasterizing the child layers.
    ///
    /// Filters sharing a `backdrop_id` sample one backdrop, as Flutter's `BackdropKey` groups
    /// them.
    pub fn push_backdrop_filter(
        &mut self,
        filter: &ImageFilter,
        blend_mode: BlendMode,
        backdrop_id: Option<u64>,
    ) {
        let backdrop = valo_backdrop(filter, backdrop_id);
        let paint = Paint {
            blend_mode,
            ..Paint::default()
        };
        self.open(|canvas| canvas.save_layer_backdrop(None, &paint, backdrop));
    }

    /// Ends the effect of the most recently pushed operation.
    ///
    /// The scope's recording becomes the [`EngineLayer`] returned, embedded in the enclosing
    /// recording: the layer that pushed keeps it and hands it to
    /// [`add_retained`](Self::add_retained) while its subtree stays unchanged. dart:ui returns
    /// the engine layer from the push; a display list is only closed at its restore.
    ///
    /// # Panics
    ///
    /// If nothing is pushed.
    pub fn pop(&mut self) -> EngineLayer {
        let enclosing = self
            .enclosing
            .pop()
            .expect("SceneBuilder::pop without a push");
        self.canvas.restore();
        let scope = std::mem::replace(&mut self.canvas, enclosing.canvas);
        self.transform = enclosing.transform;
        self.clip = enclosing.clip;
        let layer = Arc::new(scope.build());
        self.canvas.draw_display_list(&layer);
        layer
    }

    /// Places a native surface the host composites, at `rect` in the open scope's
    /// coordinates: Flutter's `addPlatformView`. Nothing is recorded in the picture; the
    /// caller paints the rectangle clear itself, at the place in paint order it wants the
    /// surface to show through.
    pub fn add_surface(&mut self, surface: Arc<dyn Surface>, rect: Rect) {
        let rect = self.in_view_coordinates(rect);
        let clip = match self.clip {
            Some(clip) => visible_part_of(rect, clip),
            None => rect,
        };
        self.surfaces.push(SurfacePlacement {
            surface,
            rect,
            scale: horizontal_scale_of(self.transform),
            clip,
        });
    }

    /// Adds a retained [`EngineLayer`] subtree from a previous frame.
    ///
    /// All the engine layers that are already in the subtree are kept as-is: the subtree is
    /// embedded exactly as it was recorded, under the scopes open now.
    pub fn add_retained(&mut self, retained_layer: &EngineLayer) {
        self.canvas.draw_display_list(retained_layer);
    }

    /// Adds a [`Picture`] to the scene.
    ///
    /// The picture is rasterized at the given `offset`. The hints are for a raster cache valo
    /// does not have; they are accepted so a layer can pass what Flutter passes.
    pub fn add_picture(
        &mut self,
        offset: Offset,
        picture: &Arc<Picture>,
        is_complex_hint: bool,
        will_change_hint: bool,
    ) {
        let _ = (is_complex_hint, will_change_hint);
        self.canvas.save();
        self.canvas
            .translate(offset.dx() as f32, offset.dy() as f32);
        self.canvas.draw_display_list(picture);
        self.canvas.restore();
    }

    /// Finishes building the scene.
    ///
    /// # Panics
    ///
    /// In debug builds, if a pushed operation was not popped.
    pub fn build(self) -> Scene {
        debug_assert!(
            self.enclosing.is_empty(),
            "SceneBuilder::build with an open push"
        );
        Scene {
            picture: Arc::new(self.canvas.build()),
            surfaces: self.surfaces,
        }
    }

    /// Opens a scope: a recording of its own, so that `pop` can hand the scope back as one
    /// list, with `push` recording the scope's own effect at its start.
    fn open(&mut self, push: impl FnOnce(&mut Canvas)) {
        let enclosing = std::mem::replace(&mut self.canvas, Canvas::new());
        self.enclosing.push(EnclosingScope {
            canvas: enclosing,
            transform: self.transform,
            clip: self.clip,
        });
        push(&mut self.canvas);
    }

    /// The open scope's coordinates are now `matrix` inside what they were: a point is
    /// taken through `matrix` first, then through the transforms pushed before it.
    fn transformed_by(&mut self, matrix: &Matrix4) {
        self.transform = self.transform.then(matrix);
    }

    /// The open scope shows nothing outside `bounds`, in its own coordinates, besides what the
    /// scopes around it already cut.
    fn clipped_to(&mut self, bounds: Rect) {
        let bounds = self.in_view_coordinates(bounds);
        self.clip = Some(match self.clip {
            Some(clip) => visible_part_of(bounds, clip),
            None => bounds,
        });
    }

    /// `rect` in the view's coordinates: the box around its corners under the open scope's
    /// transform.
    fn in_view_coordinates(&self, rect: Rect) -> Rect {
        let corners = [
            Offset::new(rect.left, rect.top),
            Offset::new(rect.right, rect.top),
            Offset::new(rect.left, rect.bottom),
            Offset::new(rect.right, rect.bottom),
        ]
        .map(|corner| transform3(self.transform, corner));
        let xs = corners.iter().map(|corner| corner.dx());
        let ys = corners.iter().map(|corner| corner.dy());
        Rect::from_ltrb(
            xs.clone().fold(f64::INFINITY, f64::min),
            ys.clone().fold(f64::INFINITY, f64::min),
            xs.fold(f64::NEG_INFINITY, f64::max),
            ys.fold(f64::NEG_INFINITY, f64::max),
        )
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

/// The rectangle `path` lies in.
fn bounds_of(path: &Path) -> Rect {
    let bounds = path.bounds();
    Rect::from_ltwh(
        f64::from(bounds.x),
        f64::from(bounds.y),
        f64::from(bounds.width),
        f64::from(bounds.height),
    )
}

/// What of `rect` lies inside `clip`; an empty rectangle on `clip`'s nearest edge when
/// nothing does.
fn visible_part_of(rect: Rect, clip: Rect) -> Rect {
    let overlap = rect.intersect(clip);
    Rect::from_ltrb(
        overlap.left,
        overlap.top,
        overlap.right.max(overlap.left),
        overlap.bottom.max(overlap.top),
    )
}

/// How long one unit along the horizontal becomes under `transform`.
fn horizontal_scale_of(transform: Matrix4) -> f64 {
    let m = transform.to_flutter_array();
    f64::from(m[0]).hypot(f64::from(m[1]))
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
    use crate::painting::{ColorFilter, flattened_ops};

    #[test]
    fn pushes_and_pops_balance() {
        let mut builder = SceneBuilder::new();
        builder.push_offset(1.0, 2.0);
        builder.push_opacity(128, Offset::ZERO);
        builder.pop();
        builder.pop();
        let _scene = builder.build();
    }

    #[test]
    fn a_popped_scope_is_the_engine_layer_embedded_in_its_parent() {
        let mut canvas = Canvas::new();
        canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), &Paint::default());
        let picture = Arc::new(canvas.build());
        let mut builder = SceneBuilder::new();
        builder.push_offset(1.0, 2.0);
        builder.add_picture(Offset::ZERO, &picture, false, false);
        let layer = builder.pop();
        builder.add_retained(&layer);
        let scene = builder.build();
        let embedded = scene
            .picture
            .ops()
            .iter()
            .filter(|op| matches!(op, valo::Op::DrawDisplayList { list, .. } if Arc::ptr_eq(list, &layer)))
            .count();
        assert_eq!(
            embedded,
            2,
            "once from the pop, once retained: {:?}",
            scene.picture.ops()
        );
    }

    /// A surface no producer feeds. The crates above the embedder use `test_support`'s,
    /// which this crate's own tests are built without.
    struct IdleSurface;

    impl Surface for IdleSurface {
        fn present(&self, _frame: crate::SurfaceFrame, _released: Box<dyn FnOnce() + Send + Sync>) {
        }

        fn open_child(&self, _opaque: bool) -> Arc<dyn Surface> {
            idle_surface()
        }
    }

    fn idle_surface() -> Arc<dyn Surface> {
        Arc::new(IdleSurface)
    }

    #[test]
    fn a_surface_is_placed_under_the_open_transforms() {
        let (inner, outer) = (idle_surface(), idle_surface());
        let mut builder = SceneBuilder::new();
        builder.push_transform(&Matrix4::scale(2.0, 2.0));
        builder.push_offset(1.0, 1.0);
        builder.add_surface(Arc::clone(&inner), Rect::from_ltwh(0.0, 0.0, 1.0, 1.0));
        builder.pop();
        builder.pop();
        // Outside every scope: the view's own coordinates.
        builder.add_surface(Arc::clone(&outer), Rect::from_ltwh(10.0, 20.0, 5.0, 5.0));
        let scene = builder.build();
        assert_eq!(
            scene.surfaces,
            [
                unclipped(inner, Rect::from_ltrb(2.0, 2.0, 4.0, 4.0), 2.0),
                unclipped(outer, Rect::from_ltwh(10.0, 20.0, 5.0, 5.0), 1.0),
            ]
        );
    }

    /// A page at half its size on a display of two pixels a point: each unit of the page's
    /// own spans one of the view's pixels, so a buffer drawn at two pixels a point shows at
    /// half its size.
    #[test]
    fn a_surface_carries_the_scale_of_the_transforms_above_it() {
        let surface = idle_surface();
        let mut builder = SceneBuilder::new();
        builder.push_transform(&Matrix4::scale(2.0, 2.0));
        builder.push_offset(10.0, 20.0);
        builder.push_transform(&Matrix4::scale(0.5, 0.5));
        builder.add_surface(
            Arc::clone(&surface),
            Rect::from_ltwh(0.0, 0.0, 390.0, 844.0),
        );
        builder.pop();
        builder.pop();
        builder.pop();
        let placement = &builder.build().surfaces[0];
        assert_eq!(placement.scale, 1.0);
        assert_eq!(placement.rect, Rect::from_ltwh(20.0, 40.0, 390.0, 844.0));
    }

    #[test]
    fn a_surface_shows_only_what_the_clips_above_it_leave() {
        let (clipped, beside) = (idle_surface(), idle_surface());
        let mut builder = SceneBuilder::new();
        builder.push_transform(&Matrix4::scale(2.0, 2.0));
        builder.push_clip_rect(Rect::from_ltwh(0.0, 0.0, 100.0, 50.0), Clip::HardEdge);
        builder.push_offset(10.0, 10.0);
        let path = {
            let mut path = crate::painting::PathBuilder::new();
            path.rrect(Rect::from_ltwh(0.0, 0.0, 200.0, 20.0).into(), 4.0);
            path.build()
        };
        builder.push_clip_path(&path, Clip::AntiAlias);
        builder.add_surface(
            Arc::clone(&clipped),
            Rect::from_ltwh(-5.0, -5.0, 300.0, 300.0),
        );
        builder.pop();
        builder.pop();
        builder.add_surface(Arc::clone(&beside), Rect::from_ltwh(0.0, 0.0, 300.0, 300.0));
        builder.pop();
        builder.pop();
        let scene = builder.build();
        // The path's bounds, (10, 10) to (210, 30), cut by the rectangle's, (0, 0) to (100, 50),
        // at two pixels to a unit.
        assert_eq!(
            scene.surfaces[0].clip,
            Rect::from_ltrb(20.0, 20.0, 200.0, 60.0)
        );
        assert_eq!(
            scene.surfaces[0].rect,
            Rect::from_ltwh(10.0, 10.0, 600.0, 600.0)
        );
        assert_eq!(
            scene.surfaces[1].clip,
            Rect::from_ltwh(0.0, 0.0, 200.0, 100.0),
            "the rectangle's clip alone, once the path's scope is closed"
        );
    }

    #[test]
    fn a_surface_outside_its_clip_shows_nothing() {
        let surface = idle_surface();
        let mut builder = SceneBuilder::new();
        builder.push_clip_rect(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0), Clip::HardEdge);
        builder.add_surface(surface, Rect::from_ltwh(200.0, 0.0, 50.0, 50.0));
        builder.pop();
        let clip = builder.build().surfaces[0].clip;
        assert!(clip.is_empty(), "{clip:?}");
    }

    #[test]
    fn a_placement_is_its_surface_its_rectangle_its_scale_and_what_shows() {
        let surface = idle_surface();
        let rect = Rect::from_ltwh(0.0, 0.0, 4.0, 4.0);
        let placement = unclipped(Arc::clone(&surface), rect, 1.0);
        assert_eq!(placement.clone(), placement);
        let elsewhere = unclipped(
            Arc::clone(&surface),
            Rect::from_ltwh(1.0, 0.0, 4.0, 4.0),
            1.0,
        );
        assert_ne!(elsewhere, placement);
        let smaller = unclipped(Arc::clone(&surface), rect, 0.5);
        assert_ne!(smaller, placement);
        let cut = SurfacePlacement {
            clip: Rect::from_ltwh(0.0, 0.0, 2.0, 4.0),
            ..placement.clone()
        };
        assert_ne!(cut, placement);
        let another_surface = unclipped(idle_surface(), rect, 1.0);
        assert_ne!(
            another_surface, placement,
            "the same rectangle is not the same surface"
        );
    }

    /// `surface` at `rect`, all of it showing, at `scale`.
    fn unclipped(surface: Arc<dyn Surface>, rect: Rect, scale: f64) -> SurfacePlacement {
        SurfacePlacement {
            surface,
            rect,
            scale,
            clip: rect,
        }
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
        flattened_ops(&builder.build().picture)
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
    #[cfg(feature = "test-support")]
    mod drawn {
        use super::*;
        use crate::test_support::drawn_pixels;

        const SIZE: u32 = 32;

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
            let pixels = drawn_pixels(&builder.build().picture, [SIZE, SIZE]);
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

        /// A picture that covers the whole scene in `color`.
        fn filled(color: Color) -> Arc<Picture> {
            let mut canvas = Canvas::new();
            let paint = Paint {
                color: color.into(),
                ..Paint::default()
            };
            canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, SIZE.into(), SIZE.into()), &paint);
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

    #[test]
    fn a_scene_without_surfaces_places_none() {
        let scene = SceneBuilder::new().build();
        assert!(scene.surfaces.is_empty());
        assert!(scene.picture.ops().is_empty());
    }

    #[test]
    #[should_panic(expected = "open push")]
    fn build_with_an_open_push_panics() {
        let mut builder = SceneBuilder::new();
        builder.push_offset(0.0, 0.0);
        let _scene = builder.build();
    }

    #[test]
    #[should_panic(expected = "without a push")]
    fn pop_without_a_push_panics() {
        SceneBuilder::new().pop();
    }
}

//! Flutter counterpart: dart:ui `SceneBuilder` / `Scene`.
//!
//! The engine's builder composites retained `EngineLayer`s; valo composites a display list, so
//! every push opens a nested recording that `pop` closes into a list of its own: that list is
//! the engine layer, embedded in the enclosing recording and handed back for the layer to keep,
//! and `add_retained` embeds it again while the subtree has not changed (see the rendering
//! crate's `PORTING.md`).

use std::fmt;
use std::sync::Arc;

use crate::geometry::{Clip, Color, Matrix4, Offset, RRect, Rect, rrect_radii_elliptical};
use crate::painting::{
    Backdrop, BlendMode, Canvas, ClipOp, FillRule, ImageFilter, Paint, Path, Picture, valo,
};

/// Flutter `Scene` — what [`SceneBuilder::build`] produces and [`crate::View::present`] takes.
pub type Scene = Picture;

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
    /// The recordings of the scopes enclosing the open one, outermost first; every `push_*`
    /// adds one and every `pop` takes one back. dart:ui asserts the same balance.
    enclosing: Vec<Canvas>,
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
            enclosing: Vec::new(),
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
    }

    /// Pushes a rectangular clip operation onto the operation stack.
    ///
    /// Rasterization outside the given rectangle is discarded.
    pub fn push_clip_rect(&mut self, rect: Rect, clip_behavior: Clip) {
        self.open(|canvas| {
            open_clip(canvas, clip_behavior, rect.into());
            canvas.clip_rect(rect, ClipOp::Intersect);
        });
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
    }

    /// Pushes a path clip operation onto the operation stack.
    ///
    /// Rasterization outside the given path is discarded.
    pub fn push_clip_path(&mut self, path: &Arc<Path>, clip_behavior: Clip) {
        self.open(|canvas| {
            open_clip(canvas, clip_behavior, path.bounds());
            canvas.clip_path(path, FillRule::NonZero, ClipOp::Intersect);
        });
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
    }

    /// Pushes a backdrop filter operation onto the operation stack.
    ///
    /// The given filter is applied to the current contents of the scene as far back as the most
    /// recent save layer and rendered back to the scene using the indicated `blend_mode` prior to
    /// rasterizing the child layers.
    ///
    /// Filters sharing a `backdrop_id` sample one backdrop, as Flutter's `BackdropKey` groups
    /// them. valo's backdrop is a blur: a colour filter in `filter` contributes nothing (see the
    /// host's `PORTING.md`).
    pub fn push_backdrop_filter(
        &mut self,
        filter: &ImageFilter,
        blend_mode: BlendMode,
        backdrop_id: Option<u64>,
    ) {
        let (sigma, bounds) = backdrop_blur(filter).unwrap_or((0.0, None));
        let mut backdrop = Backdrop::blur(sigma as f32);
        if let Some(id) = backdrop_id {
            backdrop = backdrop.shared(id);
        }
        let paint = Paint {
            blend_mode,
            ..Paint::default()
        };
        self.open(|canvas| {
            canvas.save_layer_backdrop(bounds.map(Into::into), &paint, backdrop);
        });
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
        let scope = std::mem::replace(&mut self.canvas, enclosing);
        let layer = Arc::new(scope.build());
        self.canvas.draw_display_list(&layer);
        layer
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
        self.canvas.build()
    }

    /// Opens a scope: a recording of its own, so that `pop` can hand the scope back as one
    /// list, with `push` recording the scope's own effect at its start.
    fn open(&mut self, push: impl FnOnce(&mut Canvas)) {
        let enclosing = std::mem::replace(&mut self.canvas, Canvas::new());
        self.enclosing.push(enclosing);
        push(&mut self.canvas);
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

/// The blur a valo [`Backdrop`] can sample the scene with, out of an arbitrary filter tree: its
/// sigma and the bounds it is confined to.
fn backdrop_blur(filter: &ImageFilter) -> Option<(f64, Option<Rect>)> {
    match filter {
        ImageFilter::Blur {
            sigma_x, bounds, ..
        } => Some((*sigma_x, *bounds)),
        ImageFilter::Color(_) => None,
        ImageFilter::Compose { outer, inner } => {
            backdrop_blur(inner).or_else(|| backdrop_blur(outer))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .ops()
            .iter()
            .filter(|op| matches!(op, valo::Op::DrawDisplayList { list, .. } if Arc::ptr_eq(list, &layer)))
            .count();
        assert_eq!(
            embedded,
            2,
            "once from the pop, once retained: {:?}",
            scene.ops()
        );
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

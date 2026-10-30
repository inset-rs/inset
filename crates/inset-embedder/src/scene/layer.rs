//! The nodes of a frame's layer tree, after the engine's `flow/layers`: containers that do one
//! thing to their children, the pictures the framework recorded, and the native surfaces it
//! places.
//!
//! A node never changes once built: a container the framework retains is the same `Arc` in every
//! scene that holds it, so a host compares it by pointer and a later frame walks it again. The
//! invariant a later editor must keep: nothing here draws or composites; the drawing is the
//! pictures', and what the containers do is replayed by whoever walks the tree (`slices.rs`).

use std::fmt;
use std::sync::Arc;

use crate::composition::Surface;
use crate::geometry::{Clip, Matrix4, Offset, RRect, Rect};
use crate::painting::{BlendMode, ImageFilter, Path, Picture};

/// One node of a frame's layer tree: the engine's `flow::Layer`.
#[derive(Clone, Debug)]
pub enum Layer {
    /// Children under one effect: what a `push_*` opens and `pop` closes, or a subtree a
    /// previous frame built, added again.
    Container(EngineLayer),
    /// A recorded picture at an offset: flow's `DisplayListLayer`, from `add_picture`.
    DisplayList(DisplayListLayer),
    /// A native surface at its place in paint order: flow's `PlatformViewLayer`, from
    /// `add_surface`. It draws nothing; the host places the surface and makes room for it.
    Surface(SurfaceLayer),
}

/// Two layers are equal when they are the same node or show the same: a container by pointer
/// first, so a retained subtree compares at once.
impl PartialEq for Layer {
    fn eq(&self, other: &Layer) -> bool {
        match (self, other) {
            (Layer::Container(a), Layer::Container(b)) => Arc::ptr_eq(a, b) || a == b,
            (Layer::DisplayList(a), Layer::DisplayList(b)) => a == b,
            (Layer::Surface(a), Layer::Surface(b)) => a == b,
            _ => false,
        }
    }
}

/// dart:ui's `EngineLayer`: a container of the tree, which [`pop`](crate::SceneBuilder::pop)
/// returns and [`add_retained`](crate::SceneBuilder::add_retained) adds again, as the same node,
/// while the framework's subtree has not changed.
pub type EngineLayer = Arc<ContainerLayer>;

/// flow's `ContainerLayer` and the containers derived from it: children in paint order, the
/// bottom-most first, under one effect.
#[derive(Debug, PartialEq)]
pub struct ContainerLayer {
    pub effect: Effect,
    pub children: Vec<Layer>,
}

/// What a container does to its children: which of flow's containers it is.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// flow's `ContainerLayer` itself, which does nothing: the scene's root.
    None,
    /// `TransformLayer`: the children's coordinates taken through the matrix. `push_transform`,
    /// and `push_offset` as a translation.
    Transform(Matrix4),
    /// `ClipRectLayer`: nothing of the children outside the rectangle.
    ClipRect { rect: Rect, clip_behavior: Clip },
    /// `ClipRRectLayer`: nothing of the children outside the rounded rectangle.
    ClipRRect { rrect: RRect, clip_behavior: Clip },
    /// `ClipPathLayer`: nothing of the children outside the path, filled non-zero.
    ClipPath {
        path: Arc<Path>,
        clip_behavior: Clip,
    },
    /// `OpacityLayer`: the children moved by `offset` and composited as one at `alpha`, out of
    /// 255.
    Opacity { alpha: u8, offset: Offset },
    /// `BackdropFilterLayer`: what lies beneath filtered, then blended back with `blend_mode`,
    /// before the children paint. Filters sharing a `backdrop_id` sample one backdrop.
    BackdropFilter {
        filter: ImageFilter,
        blend_mode: BlendMode,
        backdrop_id: Option<u64>,
    },
}

/// flow's `DisplayListLayer`: a picture the framework recorded, drawn at `offset`.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayListLayer {
    pub offset: Offset,
    pub picture: Arc<Picture>,
}

/// flow's `PlatformViewLayer`, with Inset's word for a platform view: a native surface the host
/// composites, at `rect` in its container's coordinates.
#[derive(Clone)]
pub struct SurfaceLayer {
    /// The producer's own handle, so a host reaches the native object behind it directly.
    pub surface: Arc<dyn Surface>,
    pub rect: Rect,
}

/// Two surface layers are equal when they put the same surface, by pointer, at the same place.
impl PartialEq for SurfaceLayer {
    fn eq(&self, other: &SurfaceLayer) -> bool {
        Arc::ptr_eq(&self.surface, &other.surface) && self.rect == other.rect
    }
}

impl fmt::Debug for SurfaceLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceLayer")
            .field("surface", &Arc::as_ptr(&self.surface).cast::<()>())
            .field("rect", &self.rect)
            .finish()
    }
}

//! Flutter counterpart: `rendering/layer.dart` (`PictureLayer`, `ContainerLayer`, `OffsetLayer`,
//! `OpacityLayer`, `TransformLayer`, `ClipRectLayer`, `ClipRRectLayer`, `ClipPathLayer`).
//!
//! Layers are data, not objects. A repaint boundary retains its recording as [`PaintItem`]s and
//! composites it through one [`CompositedLayer`]; the host composes the frame from those retained
//! pieces each time. See `PORTING.md`.

use std::sync::Arc;

use reveal_embedder::{
    Canvas, Clip, ClipOp, Color, FillRule, Matrix4, Offset, Paint, Path, Picture, RRect, Rect,
    rrect_radii_elliptical,
};
use reveal_foundation::App;

use crate::debug::debug_disable_opacity_layers;
use crate::object::AnyRenderObject;

/// The layer a repaint boundary composites its recording through.
///
/// Flutter's `OffsetLayer` and its subclasses `OpacityLayer` and `TransformLayer`.
/// [`crate::RenderObject::update_composited_layer`] returns one;
/// [`AnyRenderObject::mark_needs_composited_layer_update`] replaces it without repainting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompositedLayer {
    /// Offset from parent in the parent's coordinate system. Set by the parent's paint.
    pub offset: Offset,
    /// Which layer this is.
    pub kind: CompositedLayerKind,
}

/// The subclass of a [`CompositedLayer`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompositedLayerKind {
    /// `OffsetLayer`.
    Offset,
    /// `OpacityLayer`. `alpha` is 0 to 255; 255 composites as an `OffsetLayer`.
    Opacity {
        /// The amount to multiply into the alpha channel.
        alpha: i32,
    },
    /// `TransformLayer`.
    Transform {
        /// The matrix to apply, after `offset`.
        transform: Matrix4,
    },
}

impl CompositedLayer {
    /// Flutter's `OffsetLayer({offset})`.
    pub const fn offset_layer(offset: Offset) -> CompositedLayer {
        CompositedLayer {
            offset,
            kind: CompositedLayerKind::Offset,
        }
    }

    /// Flutter's `OpacityLayer({alpha, offset})`.
    pub const fn opacity_layer(alpha: i32, offset: Offset) -> CompositedLayer {
        CompositedLayer {
            offset,
            kind: CompositedLayerKind::Opacity { alpha },
        }
    }

    /// Flutter's `TransformLayer({transform, offset})`.
    pub const fn transform_layer(transform: Matrix4, offset: Offset) -> CompositedLayer {
        CompositedLayer {
            offset,
            kind: CompositedLayerKind::Transform { transform },
        }
    }
}

impl Default for CompositedLayer {
    /// Flutter's `OffsetLayer()`.
    fn default() -> CompositedLayer {
        CompositedLayer::offset_layer(Offset::ZERO)
    }
}

/// One retained piece of a repaint boundary's recording: Flutter's layer subclasses as data.
pub(crate) enum PaintItem {
    /// `PictureLayer`. `cache` is Flutter's `isComplexHint`: a raster-cache candidate.
    Picture { picture: Arc<Picture>, cache: bool },
    /// A child repaint boundary, composited through its own [`BoundaryLayer`].
    ChildBoundary(AnyRenderObject),
    /// `OpacityLayer` from `PaintingContext::push_opacity`. Closed by [`Pop`](Self::Pop).
    PushOpacity { alpha: i32, offset: Offset },
    /// `ClipRectLayer`.
    PushClipRect {
        clip_rect: Rect,
        clip_behavior: Clip,
    },
    /// `ClipRRectLayer`. `offset` is applied to the shape at composition.
    PushClipRRect {
        clip_rrect: RRect,
        offset: Offset,
        bounds: Rect,
        clip_behavior: Clip,
    },
    /// `ClipPathLayer`. `offset` is applied to the path at composition.
    PushClipPath {
        clip_path: Arc<Path>,
        offset: Offset,
        bounds: Rect,
        clip_behavior: Clip,
    },
    /// `TransformLayer` from `PaintingContext::push_transform`.
    PushTransform { transform: Matrix4 },
    /// Closes the innermost `Push*`.
    Pop,
}

/// A repaint boundary's layer: the composited wrapper plus the retained recording.
///
/// Flutter's `RenderObject.layer`, an `OffsetLayer` whose children are the recording.
pub struct BoundaryLayer {
    pub(crate) composited: CompositedLayer,
    pub(crate) items: Vec<PaintItem>,
    /// Flutter's `Layer.attached`: composited by an attached parent. The root layer is attached
    /// by `schedule_initial_paint`.
    pub(crate) attached: bool,
    /// The boundary whose recording composites this one. Flutter's `Layer.parent`.
    pub(crate) parent: Option<AnyRenderObject>,
}

impl BoundaryLayer {
    pub(crate) fn new(composited: CompositedLayer, attached: bool) -> BoundaryLayer {
        BoundaryLayer {
            composited,
            items: Vec::new(),
            attached,
            parent: None,
        }
    }

    /// The composited wrapper.
    pub fn composited(&self) -> CompositedLayer {
        self.composited
    }

    /// Flutter's `Layer.attached`.
    pub fn attached(&self) -> bool {
        self.attached
    }

    /// Composes the retained recording into `canvas`.
    ///
    /// Flutter's `ContainerLayer.buildScene` / `addToScene`. `RenderView.compositeFrame` calls
    /// it on the root boundary's layer.
    pub fn add_to_scene(&self, app: &App, canvas: &mut Canvas) {
        // Flutter's `OpacityLayer.addToScene`: a layer without children adds nothing.
        if self.items.is_empty() {
            return;
        }
        push_composited(canvas, &self.composited);
        for item in &self.items {
            match item {
                PaintItem::Picture { picture, cache } => {
                    if *cache {
                        canvas.draw_display_list_cached(picture);
                    } else {
                        canvas.draw_display_list(picture);
                    }
                }
                PaintItem::ChildBoundary(child) => {
                    if let Some(layer) = child.layer(app) {
                        layer.add_to_scene(app, canvas);
                    }
                }
                PaintItem::PushOpacity { alpha, offset } => push_opacity(canvas, *alpha, *offset),
                PaintItem::PushClipRect {
                    clip_rect,
                    clip_behavior,
                } => push_clip(canvas, *clip_behavior, *clip_rect, |canvas| {
                    canvas.clip_rect(*clip_rect, ClipOp::Intersect);
                }),
                PaintItem::PushClipRRect {
                    clip_rrect,
                    offset,
                    bounds,
                    clip_behavior,
                } => push_clip(canvas, *clip_behavior, *bounds, |canvas| {
                    translate(canvas, *offset);
                    canvas.clip_rrect_radii_elliptical(
                        clip_rrect.outer_rect(),
                        rrect_radii_elliptical(*clip_rrect),
                        ClipOp::Intersect,
                    );
                    translate(canvas, -*offset);
                }),
                PaintItem::PushClipPath {
                    clip_path,
                    offset,
                    bounds,
                    clip_behavior,
                } => push_clip(canvas, *clip_behavior, *bounds, |canvas| {
                    translate(canvas, *offset);
                    canvas.clip_path(clip_path, FillRule::NonZero, ClipOp::Intersect);
                    translate(canvas, -*offset);
                }),
                PaintItem::PushTransform { transform } => {
                    canvas.save();
                    canvas.concat(transform);
                }
                PaintItem::Pop => canvas.restore(),
            }
        }
        canvas.restore();
    }
}

fn translate(canvas: &mut Canvas, offset: Offset) {
    canvas.translate(offset.dx() as f32, offset.dy() as f32);
}

/// Flutter's `OffsetLayer.addToScene` / `OpacityLayer.addToScene` / `TransformLayer.addToScene`
/// up to `addChildrenToScene`. One scope; `add_to_scene` restores it.
fn push_composited(canvas: &mut Canvas, layer: &CompositedLayer) {
    match layer.kind {
        CompositedLayerKind::Offset => {
            canvas.save();
            translate(canvas, layer.offset);
        }
        CompositedLayerKind::Opacity { alpha } => push_opacity(canvas, alpha, layer.offset),
        CompositedLayerKind::Transform { transform } => {
            canvas.save();
            translate(canvas, layer.offset);
            canvas.concat(&transform);
        }
    }
}

/// `SceneBuilder.pushOpacity`, or `pushOffset` when fully opaque or opacity layers are disabled,
/// as Flutter's `OpacityLayer.addToScene` chooses.
fn push_opacity(canvas: &mut Canvas, alpha: i32, offset: Offset) {
    if alpha < 255 && !debug_disable_opacity_layers() {
        let paint = Paint {
            color: Color::from_argb(alpha, 0, 0, 0).into(),
            ..Paint::default()
        };
        canvas.save_layer(None, &paint);
    } else {
        canvas.save();
    }
    translate(canvas, offset);
}

/// One clip scope. `Clip::AntiAliasWithSaveLayer` opens a layer over `bounds`, as
/// `ClipContext.clipPathAndPaint` does.
fn push_clip(
    canvas: &mut Canvas,
    clip_behavior: Clip,
    bounds: Rect,
    clip: impl FnOnce(&mut Canvas),
) {
    match clip_behavior {
        Clip::None | Clip::HardEdge | Clip::AntiAlias => canvas.save(),
        Clip::AntiAliasWithSaveLayer => canvas.save_layer(Some(bounds.into()), &Paint::default()),
    }
    clip(canvas);
}

impl AnyRenderObject {
    /// Flutter's `Layer.attach` on this boundary's layer, then on the boundaries it composites.
    pub(crate) fn attach_layer(self, app: &mut App) {
        let Some(layer) = self.layer_mut(app) else {
            return;
        };
        if layer.attached {
            return;
        }
        layer.attached = true;
        for child in self.layer_children(app) {
            child.attach_layer(app);
        }
    }

    /// Flutter's `Layer.detach` on this boundary's layer, then on the boundaries it composites.
    pub(crate) fn detach_layer(self, app: &mut App) {
        let Some(layer) = self.layer_mut(app) else {
            return;
        };
        if !layer.attached {
            return;
        }
        layer.attached = false;
        for child in self.layer_children(app) {
            child.detach_layer(app);
        }
    }

    /// Flutter's `Layer.remove`: unlinks this boundary's layer from the recording that
    /// composites it.
    pub(crate) fn remove_layer(self, app: &mut App) {
        let Some(parent) = self.layer(app).and_then(|layer| layer.parent) else {
            return;
        };
        if let Some(parent_layer) = parent.layer_mut(app) {
            parent_layer
                .items
                .retain(|item| !matches!(item, PaintItem::ChildBoundary(child) if *child == self));
        }
        self.drop_layer_from_parent(app);
    }

    /// Flutter's `ContainerLayer.removeAllChildren` on this boundary's layer.
    pub(crate) fn remove_all_layer_children(self, app: &mut App) {
        let children = self.layer_children(app);
        if let Some(layer) = self.layer_mut(app) {
            layer.items.clear();
        }
        for child in children {
            child.drop_layer_from_parent(app);
        }
    }

    /// Flutter's `ContainerLayer._dropChild` from the child's side.
    pub(crate) fn drop_layer_from_parent(self, app: &mut App) {
        let Some(layer) = self.layer_mut(app) else {
            return;
        };
        layer.parent = None;
        if layer.attached {
            self.detach_layer(app);
        }
    }

    /// The boundaries this boundary's recording composites.
    pub(crate) fn layer_children(self, app: &App) -> Vec<AnyRenderObject> {
        self.layer(app)
            .map(|layer| {
                layer
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        PaintItem::ChildBoundary(child) => Some(*child),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::valo::Op;

    use super::*;

    fn picture() -> PaintItem {
        let mut canvas = Canvas::new();
        canvas.draw_rect(
            Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            &Paint {
                color: Color::from_argb(255, 0, 0, 0).into(),
                ..Paint::default()
            },
        );
        PaintItem::Picture {
            picture: canvas.build().into(),
            cache: false,
        }
    }

    fn compose(layer: &BoundaryLayer) -> Vec<Op> {
        let app = App::new();
        let mut canvas = Canvas::new();
        layer.add_to_scene(&app, &mut canvas);
        canvas.build().ops().to_vec()
    }

    /// `layers_test.dart`: `OpacityLayer does not push an OffsetLayer if there are no children`.
    #[test]
    fn opacity_layer_without_children_adds_nothing() {
        let layer = BoundaryLayer::new(CompositedLayer::opacity_layer(128, Offset::ZERO), true);
        assert!(compose(&layer).is_empty());
    }

    #[test]
    fn opacity_layer_pushes_a_layer_below_255_and_an_offset_at_255() {
        let mut layer = BoundaryLayer::new(CompositedLayer::opacity_layer(128, Offset::ZERO), true);
        layer.items.push(picture());
        let ops = compose(&layer);
        assert!(matches!(ops[0], Op::SaveLayer { .. }), "{ops:?}");
        assert!(
            ops.iter()
                .any(|op| matches!(op, Op::DrawDisplayList { .. }))
        );
        assert!(matches!(ops.last(), Some(Op::Restore)));

        layer.composited = CompositedLayer::opacity_layer(255, Offset::ZERO);
        let ops = compose(&layer);
        assert!(matches!(ops[0], Op::Save), "{ops:?}");
    }

    #[test]
    fn pushed_items_open_and_close_scopes() {
        let mut layer = BoundaryLayer::new(CompositedLayer::default(), true);
        layer.items.push(PaintItem::PushClipRect {
            clip_rect: Rect::from_ltwh(0.0, 0.0, 5.0, 5.0),
            clip_behavior: Clip::HardEdge,
        });
        layer.items.push(picture());
        layer.items.push(PaintItem::Pop);
        let ops = compose(&layer);
        let saves = ops.iter().filter(|op| matches!(op, Op::Save)).count();
        let restores = ops.iter().filter(|op| matches!(op, Op::Restore)).count();
        assert_eq!((saves, restores), (2, 2), "{ops:?}");
    }
}

//! Flutter counterpart: `rendering/object.dart` (`PaintingContext`).

use std::sync::Arc;

use reveal_embedder::{BlendMode, Canvas, Clip, ImageFilter, Matrix4, Offset, Path, RRect, Rect};
use reveal_foundation::App;
use reveal_painting::ClipContext;

use crate::layer::{AnnotatedRegionLayer, BackdropKey, BoundaryLayer, PaintItem};
use crate::object::AnyRenderObject;

/// A place to paint.
///
/// Rather than holding a canvas directly, render objects paint using a painting context. The
/// painting context has a [`canvas`](ClipContext::canvas), which receives the individual draw
/// operations, and also has functions for painting child render objects.
///
/// The recording is retained by the repaint boundary being painted; see `layer.rs`.
pub struct PaintingContext {
    /// The repaint boundary whose recording this context fills. Flutter's `_containerLayer`.
    boundary: AnyRenderObject,
    /// An estimate of the bounds within which the painting context's canvas will record painting
    /// commands. This can be useful for debugging.
    pub estimated_bounds: Rect,
    items: Vec<PaintItem>,
    /// Flutter's `_recorder` and `_canvas`: one object here. `Some` while recording.
    canvas: Option<Canvas>,
    is_complex_hint: bool,
    will_change_hint: bool,
}

impl PaintingContext {
    pub(crate) fn new(boundary: AnyRenderObject, estimated_bounds: Rect) -> PaintingContext {
        PaintingContext {
            boundary,
            estimated_bounds,
            items: Vec::new(),
            canvas: None,
            is_complex_hint: false,
            will_change_hint: false,
        }
    }

    /// Repaint the given render object.
    ///
    /// The render object must be attached to a [`crate::PipelineOwner`], must have a composited
    /// layer, and must be in need of painting. The render object's layer is re-used, along with
    /// any layers in the subtree that don't need to be repainted.
    pub fn repaint_composited_child(app: &mut App, child: AnyRenderObject) {
        debug_assert!(child.needs_paint(app));
        Self::repaint_composited_child_inner(app, child, false);
    }

    fn repaint_composited_child_inner(
        app: &mut App,
        child: AnyRenderObject,
        debug_also_painted_parent: bool,
    ) {
        debug_assert!(child.is_repaint_boundary(app));
        if child.layer(app).is_none() {
            debug_assert!(debug_also_painted_parent);
            let layer = child.update_composited_layer(app, None);
            child.set_layer(app, Some(BoundaryLayer::new(layer, false)));
        } else {
            debug_assert!(
                debug_also_painted_parent || child.layer(app).is_some_and(BoundaryLayer::attached)
            );
            child.remove_all_layer_children(app);
            let old = child.layer(app).expect("checked").composited;
            let updated = child.update_composited_layer(app, Some(old));
            debug_assert_eq!(updated.offset, old.offset);
            child.layer_mut(app).expect("checked").composited = updated;
        }
        child.set_needs_composited_layer_update(app, false);

        let mut child_context = PaintingContext::new(child, child.paint_bounds(app));
        child.paint_with_context(app, &mut child_context, Offset::ZERO);
        child_context.stop_recording_if_needed();
        child.layer_mut(app).expect("set above").items = child_context.items;
    }

    /// Update the composited layer of `child` without repainting its children.
    ///
    /// The render object must be attached to a [`crate::PipelineOwner`], must have a composited
    /// layer, and must be in need of a composited layer update but not in need of painting.
    pub fn update_layer_properties(app: &mut App, child: AnyRenderObject) {
        debug_assert!(child.is_repaint_boundary(app) && child.was_repaint_boundary(app));
        debug_assert!(!child.needs_paint(app));
        let old = child
            .layer(app)
            .expect("repaint boundary has a layer")
            .composited;
        let updated = child.update_composited_layer(app, Some(old));
        debug_assert_eq!(updated.offset, old.offset);
        child.layer_mut(app).expect("checked").composited = updated;
        child.set_needs_composited_layer_update(app, false);
    }

    /// Paint a child render object.
    ///
    /// If the child has its own composited layer, the child will be composited into the layer
    /// subtree associated with this painting context. Otherwise, the child will be painted into
    /// the current picture for this context.
    pub fn paint_child(&mut self, app: &mut App, child: AnyRenderObject, offset: Offset) {
        if child.is_repaint_boundary(app) {
            self.stop_recording_if_needed();
            self.composite_child(app, child, offset);
        } else if child.was_repaint_boundary(app) {
            child.set_layer(app, None);
            child.paint_with_context(app, self, offset);
        } else {
            child.paint_with_context(app, self, offset);
        }
    }

    fn composite_child(&mut self, app: &mut App, child: AnyRenderObject, offset: Offset) {
        debug_assert!(!self.is_recording());
        debug_assert!(child.is_repaint_boundary(app));

        if child.needs_paint(app) || !child.was_repaint_boundary(app) {
            Self::repaint_composited_child_inner(app, child, true);
        } else if child.needs_composited_layer_update(app) {
            Self::update_layer_properties(app, child);
        }
        child
            .layer_mut(app)
            .expect("repaint boundary has a layer")
            .composited
            .offset = offset;
        self.append_layer(app, child);
    }

    /// Adds a child boundary's layer to the recording. Flutter's `appendLayer`: the layer is
    /// removed from its previous parent first.
    fn append_layer(&mut self, app: &mut App, child: AnyRenderObject) {
        debug_assert!(!self.is_recording());
        child.remove_layer(app);
        self.items.push(PaintItem::ChildBoundary(app.retain(child)));
        if let Some(layer) = child.layer_mut(app) {
            layer.parent = Some(self.boundary);
        }
        if self
            .boundary
            .layer(app)
            .is_some_and(BoundaryLayer::attached)
        {
            child.attach_layer(app);
        }
    }

    fn is_recording(&self) -> bool {
        self.canvas.is_some()
    }

    /// Stop recording to a canvas if recording has started.
    ///
    /// Do not call this function directly: the framework calls it when a child needs to be
    /// composited into the layer tree or when a layer is pushed.
    pub(crate) fn stop_recording_if_needed(&mut self) {
        let Some(canvas) = self.canvas.take() else {
            return;
        };
        self.items.push(PaintItem::Picture {
            picture: canvas.build().into(),
            cache: self.is_complex_hint && !self.will_change_hint,
        });
        self.is_complex_hint = false;
        self.will_change_hint = false;
    }

    /// Hints that the painting in the current picture is complex and would benefit from caching.
    pub fn set_is_complex_hint(&mut self) {
        self.canvas();
        self.is_complex_hint = true;
    }

    /// Hints that the painting in the current picture is likely to change next frame.
    pub fn set_will_change_hint(&mut self) {
        self.canvas();
        self.will_change_hint = true;
    }

    /// Flutter's `pushLayer`: `item` opens a scope, `painter` records into it, `Pop` closes it.
    fn push_layer(
        &mut self,
        app: &mut App,
        item: PaintItem,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        offset: Offset,
        child_paint_bounds: Option<Rect>,
    ) {
        self.stop_recording_if_needed();
        self.items.push(item);
        let child_bounds = child_paint_bounds.unwrap_or(self.estimated_bounds);
        let bounds = std::mem::replace(&mut self.estimated_bounds, child_bounds);
        painter(app, self, offset);
        self.stop_recording_if_needed();
        self.estimated_bounds = bounds;
        self.items.push(PaintItem::Pop);
    }

    /// Clip further painting using a rectangle.
    ///
    /// `offset` is the offset from the origin of the canvas' coordinate system to the origin of
    /// the caller's coordinate system. `clip_rect` is the rectangle (in the caller's coordinate
    /// system) to use to clip the painting done by `painter`, which paints on a canvas whose
    /// origin is `offset`.
    pub fn push_clip_rect(
        &mut self,
        app: &mut App,
        offset: Offset,
        clip_rect: Rect,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        clip_behavior: Clip,
    ) {
        if clip_behavior == Clip::None {
            painter(app, self, offset);
            return;
        }
        let offset_clip_rect = clip_rect.shift(offset);
        self.push_layer(
            app,
            PaintItem::PushClipRect {
                clip_rect: offset_clip_rect,
                clip_behavior,
            },
            painter,
            offset,
            Some(offset_clip_rect),
        );
    }

    /// Clip further painting using a rounded rectangle.
    ///
    /// `bounds` is the region of the canvas (in the caller's coordinate system) into which
    /// `painter` will paint.
    pub fn push_clip_rrect(
        &mut self,
        app: &mut App,
        offset: Offset,
        bounds: Rect,
        clip_rrect: RRect,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        clip_behavior: Clip,
    ) {
        if clip_behavior == Clip::None {
            painter(app, self, offset);
            return;
        }
        let offset_bounds = bounds.shift(offset);
        self.push_layer(
            app,
            PaintItem::PushClipRRect {
                clip_rrect,
                offset,
                bounds: offset_bounds,
                clip_behavior,
            },
            painter,
            offset,
            Some(offset_bounds),
        );
    }

    /// Clip further painting using a path.
    pub fn push_clip_path(
        &mut self,
        app: &mut App,
        offset: Offset,
        bounds: Rect,
        clip_path: Arc<Path>,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        clip_behavior: Clip,
    ) {
        if clip_behavior == Clip::None {
            painter(app, self, offset);
            return;
        }
        let offset_bounds = bounds.shift(offset);
        self.push_layer(
            app,
            PaintItem::PushClipPath {
                clip_path,
                offset,
                bounds: offset_bounds,
                clip_behavior,
            },
            painter,
            offset,
            Some(offset_bounds),
        );
    }

    /// Transform further painting using a matrix.
    ///
    /// `transform` is applied in the caller's coordinate system: the painting is translated by
    /// `offset`, transformed, and translated back.
    pub fn push_transform(
        &mut self,
        app: &mut App,
        offset: Offset,
        transform: Matrix4,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
    ) {
        let (dx, dy) = (offset.dx() as f32, offset.dy() as f32);
        let effective_transform = Matrix4::translation(dx, dy)
            .then(&transform)
            .then(&Matrix4::translation(-dx, -dy));
        self.push_layer(
            app,
            PaintItem::PushTransform {
                transform: effective_transform,
            },
            painter,
            offset,
            None,
        );
    }

    /// Annotate further painting with a value [`BoundaryLayer::find`] can answer.
    ///
    /// Flutter's `pushLayer(AnnotatedRegionLayer(...), painter, offset)`. The layer paints
    /// nothing of its own, so `painter` records at `offset` unchanged; the layer's own
    /// `offset` only shifts the rectangle its `size` clips the search to.
    pub fn push_annotated_region(
        &mut self,
        app: &mut App,
        layer: AnnotatedRegionLayer,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        offset: Offset,
    ) {
        self.push_layer(
            app,
            PaintItem::PushAnnotatedRegion(layer),
            painter,
            offset,
            None,
        );
    }

    /// Blend further painting with an alpha value.
    ///
    /// `alpha` is 0 to 255. `painter` paints at `Offset::ZERO`; the layer applies `offset`.
    pub fn push_opacity(
        &mut self,
        app: &mut App,
        offset: Offset,
        alpha: i32,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
    ) {
        self.push_layer(
            app,
            PaintItem::PushOpacity { alpha, offset },
            painter,
            Offset::ZERO,
            None,
        );
    }
}

impl PaintingContext {
    /// Blur what is already painted under `bounds` and paint `painter` on top: Flutter's
    /// `pushLayer(BackdropFilterLayer(filter, blendMode, backdropKey), painter, offset)`.
    ///
    /// `bounds` is the filtered render object's paint bounds in the caller's coordinate
    /// system; a filter carrying its own bounds wins.
    #[allow(clippy::too_many_arguments)]
    pub fn push_backdrop_filter(
        &mut self,
        app: &mut App,
        offset: Offset,
        bounds: Rect,
        filter: ImageFilter,
        blend_mode: BlendMode,
        backdrop_key: Option<BackdropKey>,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
    ) {
        self.push_layer(
            app,
            PaintItem::PushBackdropFilter {
                filter,
                blend_mode,
                backdrop_key,
                bounds,
                offset,
            },
            painter,
            Offset::ZERO,
            None,
        );
    }
}

impl ClipContext for PaintingContext {
    /// The canvas on which to paint. Starts recording a new picture on first use.
    fn canvas(&mut self) -> &mut Canvas {
        self.canvas.get_or_insert_with(Canvas::new)
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_embedder::{Color, Paint, Size, valo::Op};

    use super::*;
    use crate::box_::{AnyRenderBox, BoxConstraints, RenderBox, RenderBoxData};
    use crate::layer::{CompositedLayer, CompositedLayerKind};
    use crate::object::{RenderHandle, RenderObject, RenderObjectData};
    use reveal_foundation::Handle;

    use crate::pipeline_owner::PipelineOwner;

    /// Paints one rect and counts its paints. A repaint boundary when told to be, compositing
    /// through an `OpacityLayer` with `alpha`.
    struct Leaf {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        repaint_boundary: bool,
        alpha: i32,
        paints: Rc<Cell<u32>>,
    }

    impl RenderObject for Leaf {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let size = self.constraints(app).smallest();
            self.set_size(app, size);
        }

        fn is_repaint_boundary(self: RenderHandle<Self>, app: &App) -> bool {
            self.get(app).repaint_boundary
        }

        fn update_composited_layer(
            self: RenderHandle<Self>,
            app: &mut App,
            old_layer: Option<CompositedLayer>,
        ) -> CompositedLayer {
            let offset = old_layer.map_or(Offset::ZERO, |layer| layer.offset);
            CompositedLayer::opacity_layer(self.get(app).alpha, offset)
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
        ) {
            let paints = Rc::clone(&self.get(app).paints);
            paints.set(paints.get() + 1);
            let paint = Paint {
                color: Color::from_argb(255, 0, 0, 0).into(),
                ..Paint::default()
            };
            context.canvas().draw_rect(offset & self.size(app), &paint);
        }
    }

    impl RenderBox for Leaf {
        crate::render_box_accessors!();

        /// Flutter's `TestRenderObject(allowPaintBounds: true)`: paint bounds before layout.
        fn paint_bounds(self: RenderHandle<Self>, app: &App) -> Rect {
            self.render_box_data(app)
                .size
                .map_or(Rect::ZERO, |size| Offset::ZERO & size)
        }
    }

    /// A repaint boundary with one child, painted at the origin unless `paints_child` is off.
    struct Parent {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        child: Option<AnyRenderBox>,
        paints_child: bool,
        paints: Rc<Cell<u32>>,
    }

    impl RenderObject for Parent {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            if let Some(child) = self.get(app).child {
                child.layout(app, BoxConstraints::tight(Size::new(50.0, 50.0)), false);
            }
            self.set_size(app, Size::new(100.0, 100.0));
        }

        fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
            true
        }

        fn visit_children(
            self: RenderHandle<Self>,
            app: &App,
            visitor: &mut dyn FnMut(AnyRenderObject),
        ) {
            if let Some(child) = self.get(app).child {
                visitor(child.as_object());
            }
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
        ) {
            let paints = Rc::clone(&self.get(app).paints);
            paints.set(paints.get() + 1);
            if let Some(child) = self.get(app).child
                && self.get(app).paints_child
            {
                context.paint_child(app, child.as_object(), offset);
            }
        }
    }

    impl RenderBox for Parent {
        crate::render_box_accessors!();
    }

    struct Tree {
        owner: Handle<PipelineOwner>,
        parent: RenderHandle<Parent>,
        leaf: RenderHandle<Leaf>,
        parent_paints: Rc<Cell<u32>>,
        leaf_paints: Rc<Cell<u32>>,
    }

    fn tree(app: &mut App, leaf_is_boundary: bool) -> Tree {
        let owner = PipelineOwner::new(app, None);
        let leaf_paints = Rc::new(Cell::new(0));
        let parent_paints = Rc::new(Cell::new(0));
        let leaf = RenderHandle::new_box(
            app,
            Leaf {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                repaint_boundary: leaf_is_boundary,
                alpha: 255,
                paints: Rc::clone(&leaf_paints),
            },
        );
        let parent = RenderHandle::new_box(
            app,
            Parent {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: Some(leaf.as_box()),
                paints_child: true,
                paints: Rc::clone(&parent_paints),
            },
        );
        parent.adopt_child(app, leaf.as_object());
        owner.set_root_node(app, Some(parent.as_object()));
        parent.schedule_initial_layout(app);
        parent
            .as_object()
            .schedule_initial_paint(app, CompositedLayer::default());
        Tree {
            owner,
            parent,
            leaf,
            parent_paints,
            leaf_paints,
        }
    }

    fn frame(app: &mut App, owner: Handle<PipelineOwner>) {
        owner.flush_layout(app);
        owner.flush_paint(app);
    }

    fn paints(tree: &Tree) -> (u32, u32) {
        (tree.parent_paints.get(), tree.leaf_paints.get())
    }

    fn leaf_layer_attached(app: &App, tree: &Tree) -> bool {
        tree.leaf
            .as_object()
            .debug_layer(app)
            .expect("leaf has a layer")
            .attached()
    }

    /// `object_test.dart`: `nodesNeedingPaint updated with paint changes`.
    #[test]
    fn nodes_needing_paint_updated_with_paint_changes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let owner = PipelineOwner::new(&mut app, None);
        let node = RenderHandle::new_box(
            &mut app,
            Leaf {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                repaint_boundary: true,
                alpha: 255,
                paints: Rc::new(Cell::new(0)),
            },
        );
        node.as_object().attach(&mut app, owner);
        assert!(owner.nodes_needing_paint(&app).is_empty());

        node.mark_needs_paint(&mut app);
        node.as_object()
            .schedule_initial_paint(&mut app, CompositedLayer::default());
        assert!(owner.nodes_needing_paint(&app).contains(&node.as_object()));

        owner.flush_paint(&mut app);
        assert!(owner.nodes_needing_paint(&app).is_empty());
    }

    #[test]
    fn child_boundary_repaints_without_its_parent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tree = tree(&mut app, true);
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (1, 1));
        assert!(leaf_layer_attached(&app, &tree));
        assert_eq!(
            tree.parent.as_object().layer_children(&app),
            vec![tree.leaf.as_object()]
        );

        tree.leaf.mark_needs_paint(&mut app);
        assert_eq!(
            tree.owner.nodes_needing_paint(&app),
            vec![tree.leaf.as_object()]
        );
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (1, 2));
        assert_eq!(
            tree.parent.as_object().layer_children(&app),
            vec![tree.leaf.as_object()]
        );
    }

    #[test]
    fn composited_layer_update_does_not_repaint() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tree = tree(&mut app, true);
        frame(&mut app, tree.owner);

        tree.leaf.get_mut(&mut app).alpha = 128;
        tree.leaf.mark_needs_composited_layer_update(&mut app);
        assert!(
            tree.leaf
                .as_object()
                .debug_needs_composited_layer_update(&app)
        );
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (1, 1));
        let layer = tree.leaf.as_object().debug_layer(&app).expect("layer");
        assert_eq!(
            layer.composited().kind,
            CompositedLayerKind::Opacity { alpha: 128 }
        );
    }

    /// `layers_test.dart`: `non-painted layers are detached`.
    #[test]
    fn unpainted_boundary_is_detached_and_skipped() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tree = tree(&mut app, true);
        frame(&mut app, tree.owner);
        assert!(leaf_layer_attached(&app, &tree));

        tree.parent.get_mut(&mut app).paints_child = false;
        tree.parent.mark_needs_paint(&mut app);
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (2, 1));
        assert!(!leaf_layer_attached(&app, &tree));

        tree.leaf.mark_needs_paint(&mut app);
        frame(&mut app, tree.owner);
        assert_eq!(
            paints(&tree),
            (2, 1),
            "a detached boundary is not repainted"
        );
        assert!(tree.leaf.as_object().debug_needs_paint(&app));

        tree.parent.get_mut(&mut app).paints_child = true;
        tree.parent.mark_needs_paint(&mut app);
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (3, 2));
        assert!(leaf_layer_attached(&app, &tree));
    }

    #[test]
    fn non_boundary_child_paints_into_the_parent_picture() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tree = tree(&mut app, false);
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (1, 1));
        assert!(tree.parent.as_object().layer_children(&app).is_empty());
        assert!(tree.leaf.as_object().debug_layer(&app).is_none());

        tree.leaf.mark_needs_paint(&mut app);
        assert_eq!(
            tree.owner.nodes_needing_paint(&app),
            vec![tree.parent.as_object()]
        );
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (2, 2));
    }

    #[test]
    fn composing_the_root_draws_the_child_boundary_through_its_layer() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tree = tree(&mut app, true);
        tree.leaf.get_mut(&mut app).alpha = 128;
        frame(&mut app, tree.owner);

        let mut canvas = Canvas::new();
        tree.parent
            .as_object()
            .debug_layer(&app)
            .expect("root layer")
            .add_to_scene(&app, &mut canvas);
        let ops = canvas.build().ops().to_vec();
        let pictures = ops
            .iter()
            .filter(|op| matches!(op, Op::DrawDisplayList { .. }))
            .count();
        assert_eq!(pictures, 1, "{ops:?}");
        assert!(
            ops.iter().any(|op| matches!(op, Op::SaveLayer { .. })),
            "the child's OpacityLayer: {ops:?}"
        );
    }
}

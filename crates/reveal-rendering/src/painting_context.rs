//! Flutter counterpart: `rendering/object.dart` (`PaintingContext`).

use std::sync::Arc;

use reveal_embedder::{Canvas, Clip, Matrix4, Offset, Path, PathBuilder, RRect, Rect};
use reveal_foundation::{App, Handle};
use reveal_painting::{ClipContext, inverse_transform_rect};

use crate::layer::{
    AnyContainerLayer, AnyLayer, ClipPathLayer, ClipRRectLayer, ClipRectLayer, CompositionCallback,
    ContainerLayer, ErasedLayer, LayerHandle, OffsetLayerMixin, OpacityLayer, PictureLayer,
    TransformLayer,
};
use crate::object::AnyRenderObject;

/// A place to paint.
///
/// Rather than holding a canvas directly, [`crate::RenderObject`]s paint using a painting
/// context. The painting context has a [`Canvas`](ClipContext::canvas), which receives the
/// individual draw operations, and also has functions for painting child
/// render objects.
///
/// When painting a child render object, the canvas held by the painting context
/// can change because the draw operations issued before and after painting the
/// child might be recorded in separate compositing layers. For this reason, do
/// not hold a reference to the canvas across operations that might paint
/// child render objects.
///
/// New [`PaintingContext`] objects are created automatically when using
/// [`PaintingContext::repaint_composited_child`] and [`push_layer`](Self::push_layer).
pub struct PaintingContext {
    container_layer: AnyContainerLayer,
    /// An estimate of the bounds within which the painting context's [`canvas`](ClipContext::canvas)
    /// will record painting commands. This can be useful for debugging.
    ///
    /// The canvas will allow painting outside these bounds.
    ///
    /// The [`estimated_bounds`](Self::estimated_bounds) rectangle is in the canvas coordinate
    /// system.
    pub estimated_bounds: Rect,
    canvas: Option<Canvas>,
    is_complex_hint: bool,
    will_change_hint: bool,
}

impl PaintingContext {
    /// Creates a painting context.
    ///
    /// Typically only called by [`repaint_composited_child`](Self::repaint_composited_child)
    /// and [`push_layer`](Self::push_layer).
    pub fn new(container_layer: AnyContainerLayer, estimated_bounds: Rect) -> PaintingContext {
        PaintingContext {
            container_layer,
            estimated_bounds,
            canvas: None,
            is_complex_hint: false,
            will_change_hint: false,
        }
    }

    /// Repaint the given render object.
    ///
    /// The render object must be attached to a [`crate::PipelineOwner`], must have a
    /// composited layer, and must be in need of painting. The render object's
    /// layer, if any, is re-used, along with any layers in the subtree that don't
    /// need to be repainted.
    pub fn repaint_composited_child(app: &mut App, child: AnyRenderObject) {
        debug_assert!(child.needs_paint(app));
        Self::repaint_composited_child_inner(app, child, false, None);
    }

    fn repaint_composited_child_inner(
        app: &mut App,
        child: AnyRenderObject,
        debug_also_painted_parent: bool,
        child_context: Option<PaintingContext>,
    ) {
        debug_assert!(child.is_repaint_boundary(app));
        let child_layer = if let Some(child_layer) =
            child.layer(app).and_then(|layer| layer.as_offset_layer())
        {
            debug_assert!(debug_also_painted_parent || child_layer.as_layer().attached(app));
            let debug_old_offset = if cfg!(debug_assertions) {
                Some(child_layer.offset(app))
            } else {
                None
            };
            child_layer.as_container_layer().remove_all_children(app);
            let updated_layer = child.update_composited_layer(app, Some(child_layer));
            debug_assert_eq!(
                updated_layer, child_layer,
                "{child:?} created a new layer instance {updated_layer:?} instead of reusing the \
                 existing layer {child_layer:?}. See the documentation of RenderObject.updateCompositedLayer \
                 for more information on how to correctly implement this method."
            );
            debug_assert_eq!(debug_old_offset, Some(updated_layer.offset(app)));
            child_layer
        } else {
            debug_assert!(debug_also_painted_parent);
            debug_assert!(child.layer(app).is_none());

            // Not using the `layer` setter because the setter asserts that we not
            // replace the layer for repaint boundaries. That assertion does not
            // apply here because this is exactly the place designed to create a
            // layer for repaint boundaries.
            let layer = child.update_composited_layer(app, None);
            LayerHandle::set_layer(
                app,
                |app| &mut child.data_mut(app).layer,
                Some(layer.as_container_layer()),
            );
            layer
        };
        child.set_needs_composited_layer_update(app, false);
        debug_assert_eq!(Some(child_layer.as_container_layer()), child.layer(app));
        debug_assert!(
            child
                .layer(app)
                .is_some_and(|layer| layer.as_offset_layer().is_some())
        );

        let mut child_context = child_context.unwrap_or_else(|| {
            PaintingContext::new(child_layer.as_container_layer(), child.paint_bounds(app))
        });
        child.paint_with_context(app, &mut child_context, Offset::ZERO);

        // Double-check that the paint method did not replace the layer (the first
        // check is done in the [layer] setter itself).
        debug_assert_eq!(Some(child_layer.as_container_layer()), child.layer(app));
        child_context.stop_recording_if_needed(app);
    }

    /// Update the composited layer of `child` without repainting its children.
    ///
    /// The render object must be attached to a [`crate::PipelineOwner`], must have a
    /// composited layer, and must be in need of a composited layer update but
    /// not in need of painting. The render object's layer is re-used, and none
    /// of its children are repaint or their layers updated.
    pub fn update_layer_properties(app: &mut App, child: AnyRenderObject) {
        debug_assert!(child.is_repaint_boundary(app) && child.was_repaint_boundary(app));
        debug_assert!(!child.needs_paint(app));
        debug_assert!(child.layer(app).is_some());

        let child_layer = child
            .layer(app)
            .expect("checked")
            .as_offset_layer()
            .expect("a repaint boundary's layer is an OffsetLayer");
        let debug_old_offset = if cfg!(debug_assertions) {
            Some(child_layer.offset(app))
        } else {
            None
        };
        let updated_layer = child.update_composited_layer(app, Some(child_layer));
        debug_assert_eq!(
            updated_layer, child_layer,
            "{child:?} created a new layer instance {updated_layer:?} instead of reusing the \
             existing layer {child_layer:?}. See the documentation of RenderObject.updateCompositedLayer \
             for more information on how to correctly implement this method."
        );
        debug_assert_eq!(debug_old_offset, Some(updated_layer.offset(app)));
        child.set_needs_composited_layer_update(app, false);
    }

    /// Paint a child [`crate::RenderObject`].
    ///
    /// If the child has its own composited layer, the child will be composited
    /// into the layer subtree associated with this painting context. Otherwise,
    /// the child will be painted into the current PictureLayer for this context.
    pub fn paint_child(&mut self, app: &mut App, child: AnyRenderObject, offset: Offset) {
        if child.is_repaint_boundary(app) {
            self.stop_recording_if_needed(app);
            self.composite_child(app, child, offset);
        } else if child.was_repaint_boundary(app) {
            debug_assert!(
                child
                    .layer(app)
                    .is_some_and(|layer| layer.as_offset_layer().is_some())
            );
            LayerHandle::set_layer(app, |app| &mut child.data_mut(app).layer, None);
            child.paint_with_context(app, self, offset);
        } else {
            child.paint_with_context(app, self, offset);
        }
    }

    fn composite_child(&mut self, app: &mut App, child: AnyRenderObject, offset: Offset) {
        debug_assert!(!self.is_recording());
        debug_assert!(child.is_repaint_boundary(app));

        if child.needs_paint(app) || !child.was_repaint_boundary(app) {
            Self::repaint_composited_child_inner(app, child, true, None);
        } else if child.needs_composited_layer_update(app) {
            Self::update_layer_properties(app, child);
        }
        debug_assert!(
            child
                .layer(app)
                .is_some_and(|layer| layer.as_offset_layer().is_some())
        );
        let child_offset_layer = child
            .layer(app)
            .expect("checked")
            .as_offset_layer()
            .expect("a repaint boundary's layer is an OffsetLayer");
        child_offset_layer.set_offset(app, offset);
        self.append_layer(app, child_offset_layer.as_layer());
    }

    /// Adds a layer to the recording requiring that the recording is already
    /// stopped.
    ///
    /// Do not call this function directly: call [`add_layer`](Self::add_layer) or
    /// [`push_layer`](Self::push_layer) instead. This function is called internally when all
    /// layers not generated from the canvas are added.
    fn append_layer(&mut self, app: &mut App, layer: AnyLayer) {
        debug_assert!(!self.is_recording());
        layer.remove(app);
        self.container_layer.append(app, layer);
    }

    fn is_recording(&self) -> bool {
        self.canvas.is_some()
    }

    fn start_recording(&mut self) {
        debug_assert!(!self.is_recording());
        self.canvas = Some(Canvas::new());
        self.is_complex_hint = false;
        self.will_change_hint = false;
    }

    /// Adds a [`CompositionCallback`] for the current [`AnyContainerLayer`] used by this
    /// context.
    pub fn add_composition_callback(
        &self,
        app: &mut App,
        callback: CompositionCallback,
    ) -> Box<dyn FnOnce(&mut App)> {
        self.container_layer
            .as_layer()
            .add_composition_callback(app, callback)
    }

    /// Stop recording to a canvas if recording has started.
    ///
    /// Do not call this function directly: functions in this class will call
    /// this method as needed. This function is called internally to ensure that
    /// recording is stopped before adding layers or finalizing the results of a
    /// paint.
    ///
    /// Creates and appends the [`PictureLayer`] here rather than in
    /// [`start_recording`](Self::start_recording): [`ClipContext::canvas`] has no `App`. See
    /// `PORTING.md`.
    pub fn stop_recording_if_needed(&mut self, app: &mut App) {
        if !self.is_recording() {
            return;
        }
        let canvas = self.canvas.take().expect("recording");
        let picture = PictureLayer::new(app, self.estimated_bounds);
        picture.set_picture(app, Some(Arc::new(canvas.build())));
        picture.set_is_complex_hint(app, self.is_complex_hint);
        picture.set_will_change_hint(app, self.will_change_hint);
        self.is_complex_hint = false;
        self.will_change_hint = false;
        self.container_layer.append(app, picture.as_layer());
    }

    /// Hints that the painting in the current layer is complex and would benefit
    /// from caching.
    pub fn set_is_complex_hint(&mut self) {
        if self.canvas.is_none() {
            self.start_recording();
        }
        self.is_complex_hint = true;
    }

    /// Hints that the painting in the current layer is likely to change next frame.
    pub fn set_will_change_hint(&mut self) {
        if self.canvas.is_none() {
            self.start_recording();
        }
        self.will_change_hint = true;
    }

    /// Adds a composited leaf layer to the recording.
    ///
    /// After calling this function, the [`canvas`](ClipContext::canvas) property will change to
    /// refer to a new [`Canvas`] that draws on top of the given layer.
    pub fn add_layer(&mut self, app: &mut App, layer: AnyLayer) {
        self.stop_recording_if_needed(app);
        self.append_layer(app, layer);
    }

    /// Appends the given layer to the recording, and calls the `painter` callback
    /// with that layer, providing the `child_paint_bounds` as the estimated paint
    /// bounds of the child. The `child_paint_bounds` can be used for debugging but
    /// have no effect on painting.
    ///
    /// The given layer must be an unattached orphan. (Providing a newly created
    /// object, rather than reusing an existing layer, satisfies that
    /// requirement.)
    ///
    /// The `offset` is the offset to pass to the `painter`. In particular, it is
    /// not an offset applied to the layer itself. Layers conceptually by default
    /// have no position or size, though they can transform their contents. For
    /// example, an [`crate::OffsetLayer`] applies an offset to its children.
    ///
    /// If the `child_paint_bounds` are not specified then the current layer's paint
    /// bounds are used. This is appropriate if the child layer does not apply any
    /// transformation or clipping to its contents. The `child_paint_bounds`, if
    /// specified, must be in the coordinate system of the new layer (i.e. as seen
    /// by its children after it applies whatever transform to its contents), and
    /// should not go outside the current layer's paint bounds.
    pub fn push_layer(
        &mut self,
        app: &mut App,
        child_layer: AnyContainerLayer,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        offset: Offset,
        child_paint_bounds: Option<Rect>,
    ) {
        // If a layer is being reused, it may already contain children. We remove
        // them so that `painter` can add children that are relevant for this frame.
        if child_layer.has_children(app) {
            child_layer.remove_all_children(app);
        }
        self.stop_recording_if_needed(app);
        self.append_layer(app, child_layer.as_layer());
        let mut child_context = self.create_child_context(
            child_layer,
            child_paint_bounds.unwrap_or(self.estimated_bounds),
        );

        painter(app, &mut child_context, offset);
        child_context.stop_recording_if_needed(app);
    }

    /// Creates a painting context configured to paint into `child_layer`.
    ///
    /// The `bounds` are estimated paint bounds for debugging purposes.
    fn create_child_context(
        &self,
        child_layer: AnyContainerLayer,
        bounds: Rect,
    ) -> PaintingContext {
        PaintingContext::new(child_layer, bounds)
    }

    /// Clip further painting using a rectangle.
    ///
    /// The `needs_compositing` argument specifies whether the child needs
    /// compositing. Typically this matches the value of
    /// [`AnyRenderObject::needs_compositing`] for the caller. If false, this method
    /// returns `None`, indicating that a layer is no longer necessary. If a render
    /// object calling this method stores the `old_layer` in its
    /// [`AnyRenderObject::layer`] field, it should set that field to `None`.
    ///
    /// When `needs_compositing` is false, this method will use a more efficient
    /// way to apply the layer effect than actually creating a layer.
    ///
    /// The `offset` argument is the offset from the origin of the canvas'
    /// coordinate system to the origin of the caller's coordinate system.
    ///
    /// The `clip_rect` is the rectangle (in the caller's coordinate system) to use
    /// to clip the painting done by `painter`. It should not include the
    /// `offset`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_clip_rect(
        &mut self,
        app: &mut App,
        needs_compositing: bool,
        offset: Offset,
        clip_rect: Rect,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        clip_behavior: Clip,
        old_layer: Option<Handle<ClipRectLayer>>,
    ) -> Option<Handle<ClipRectLayer>> {
        if clip_behavior == Clip::None {
            painter(app, self, offset);
            return None;
        }
        let offset_clip_rect = clip_rect.shift(offset);
        if needs_compositing {
            let layer = old_layer.unwrap_or_else(|| ClipRectLayer::new(app));
            layer.set_clip_rect(app, Some(offset_clip_rect));
            layer.set_clip_behavior(app, clip_behavior);
            self.push_layer(
                app,
                layer.as_container_layer(),
                painter,
                offset,
                Some(offset_clip_rect),
            );
            Some(layer)
        } else {
            self.clip_rect_and_paint(
                offset_clip_rect,
                clip_behavior,
                offset_clip_rect,
                |context| painter(app, context, offset),
            );
            None
        }
    }

    /// Clip further painting using a rounded rectangle.
    #[allow(clippy::too_many_arguments)]
    pub fn push_clip_rrect(
        &mut self,
        app: &mut App,
        needs_compositing: bool,
        offset: Offset,
        bounds: Rect,
        clip_rrect: RRect,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        clip_behavior: Clip,
        old_layer: Option<Handle<ClipRRectLayer>>,
    ) -> Option<Handle<ClipRRectLayer>> {
        if clip_behavior == Clip::None {
            painter(app, self, offset);
            return None;
        }
        let offset_bounds = bounds.shift(offset);
        let offset_clip_rrect = clip_rrect.shift(offset);
        if needs_compositing {
            let layer = old_layer.unwrap_or_else(|| ClipRRectLayer::new(app));
            layer.set_clip_rrect(app, Some(offset_clip_rrect));
            layer.set_clip_behavior(app, clip_behavior);
            self.push_layer(
                app,
                layer.as_container_layer(),
                painter,
                offset,
                Some(offset_bounds),
            );
            Some(layer)
        } else {
            self.clip_rrect_and_paint(offset_clip_rrect, clip_behavior, offset_bounds, |context| {
                painter(app, context, offset)
            });
            None
        }
    }

    /// Clip further painting using a path.
    #[allow(clippy::too_many_arguments)]
    pub fn push_clip_path(
        &mut self,
        app: &mut App,
        needs_compositing: bool,
        offset: Offset,
        bounds: Rect,
        clip_path: Arc<Path>,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        clip_behavior: Clip,
        old_layer: Option<Handle<ClipPathLayer>>,
    ) -> Option<Handle<ClipPathLayer>> {
        if clip_behavior == Clip::None {
            painter(app, self, offset);
            return None;
        }
        let offset_bounds = bounds.shift(offset);
        let offset_clip_path = shift_path(&clip_path, offset);
        if needs_compositing {
            let layer = old_layer.unwrap_or_else(|| ClipPathLayer::new(app));
            layer.set_clip_path(app, Some(offset_clip_path));
            layer.set_clip_behavior(app, clip_behavior);
            self.push_layer(
                app,
                layer.as_container_layer(),
                painter,
                offset,
                Some(offset_bounds),
            );
            Some(layer)
        } else {
            self.clip_path_and_paint(&offset_clip_path, clip_behavior, offset_bounds, |context| {
                painter(app, context, offset)
            });
            None
        }
    }

    /// Transform further painting using a matrix.
    ///
    /// The `offset` argument is the offset to pass to `painter` and the offset to
    /// the origin used by `transform`.
    ///
    /// The `transform` argument is the [`Matrix4`] with which to transform the
    /// coordinate system while calling `painter`. It should not include `offset`.
    /// It is applied effectively after applying `offset`.
    pub fn push_transform(
        &mut self,
        app: &mut App,
        needs_compositing: bool,
        offset: Offset,
        transform: Matrix4,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        old_layer: Option<Handle<TransformLayer>>,
    ) -> Option<Handle<TransformLayer>> {
        let (dx, dy) = (offset.dx() as f32, offset.dy() as f32);
        let effective_transform = Matrix4::translation(dx, dy)
            .then(&transform)
            .then(&Matrix4::translation(-dx, -dy));
        if needs_compositing {
            let layer = old_layer.unwrap_or_else(|| TransformLayer::new(app));
            layer.set_transform(app, effective_transform);
            let child_paint_bounds =
                inverse_transform_rect(effective_transform, self.estimated_bounds);
            self.push_layer(
                app,
                layer.as_container_layer(),
                painter,
                offset,
                Some(child_paint_bounds),
            );
            Some(layer)
        } else {
            self.canvas().save();
            self.canvas().concat(&effective_transform);
            painter(app, self, offset);
            self.canvas().restore();
            None
        }
    }

    /// Blend further painting with an alpha value.
    ///
    /// The `offset` argument indicates an offset to apply to all the children
    /// (the rendering created by `painter`).
    ///
    /// The `alpha` argument is the alpha value to use when blending the painting
    /// done by `painter`. An alpha value of 0 means the painting is fully
    /// transparent and an alpha value of 255 means the painting is fully opaque.
    pub fn push_opacity(
        &mut self,
        app: &mut App,
        offset: Offset,
        alpha: i32,
        painter: impl FnOnce(&mut App, &mut PaintingContext, Offset),
        old_layer: Option<Handle<OpacityLayer>>,
    ) -> Handle<OpacityLayer> {
        let layer = old_layer.unwrap_or_else(|| OpacityLayer::new(app));
        layer.set_alpha(app, alpha);
        layer.set_offset(app, offset);
        self.push_layer(app, layer.as_container_layer(), painter, Offset::ZERO, None);
        layer
    }
}

fn shift_path(path: &Arc<Path>, offset: Offset) -> Arc<Path> {
    if offset == Offset::ZERO {
        return Arc::clone(path);
    }
    let mut builder = PathBuilder::new();
    builder.append(
        path,
        &Matrix4::translation(offset.dx() as f32, offset.dy() as f32),
    );
    builder.build()
}

impl ClipContext for PaintingContext {
    /// The canvas on which to paint.
    ///
    /// The current canvas can change whenever you paint a child using this
    /// context, which means it's fragile to hold a reference to the canvas
    /// returned by this getter.
    fn canvas(&mut self) -> &mut Canvas {
        if self.canvas.is_none() {
            self.start_recording();
        }
        self.canvas.as_mut().expect("recording")
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_embedder::{Color, Paint, SceneBuilder, Size, valo::Op};

    use super::*;
    use crate::box_::{AnyRenderBox, BoxConstraints, RenderBox, RenderBoxData};
    use crate::layer::{AnyOffsetLayer, ContainerLayer, ErasedLayer, OffsetLayer, OpacityLayer};
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
            old_layer: Option<AnyOffsetLayer>,
        ) -> AnyOffsetLayer {
            debug_assert!(self.is_repaint_boundary(app));
            let layer = match old_layer {
                Some(old) => app
                    .handle::<OpacityLayer>(old.id())
                    .expect("Leaf reuses an OpacityLayer"),
                None => OpacityLayer::new(app),
            };
            layer.set_alpha(app, self.get(app).alpha);
            layer.as_offset_layer()
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
        schedule_root_paint(app, parent.as_object());
        Tree {
            owner,
            parent,
            leaf,
            parent_paints,
            leaf_paints,
        }
    }

    fn schedule_root_paint(app: &mut App, node: AnyRenderObject) {
        let root = OffsetLayer::new(app, Offset::ZERO);
        root.as_layer().attach(app, node.id());
        node.schedule_initial_paint(app, root.as_container_layer());
    }

    fn frame(app: &mut App, owner: Handle<PipelineOwner>) {
        owner.flush_layout(app);
        owner.flush_compositing_bits(app);
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
            .as_layer()
            .attached(app)
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
        let root = OpacityLayer::new(&mut app);
        root.as_layer().attach(&mut app, node.as_object().id());
        node.as_object()
            .schedule_initial_paint(&mut app, root.as_container_layer());
        assert!(owner.nodes_needing_paint(&app).contains(&node.as_object()));

        owner.flush_compositing_bits(&mut app);
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
        let parent_layer = tree.parent.as_object().debug_layer(&app).expect("parent");
        let leaf_layer = tree.leaf.as_object().debug_layer(&app).expect("leaf");
        assert_eq!(leaf_layer.as_layer().parent(&app), Some(parent_layer));

        tree.leaf.mark_needs_paint(&mut app);
        assert_eq!(
            tree.owner.nodes_needing_paint(&app),
            vec![tree.leaf.as_object()]
        );
        frame(&mut app, tree.owner);
        assert_eq!(paints(&tree), (1, 2));
        let parent_layer = tree.parent.as_object().debug_layer(&app).expect("parent");
        let leaf_layer = tree.leaf.as_object().debug_layer(&app).expect("leaf");
        assert_eq!(leaf_layer.as_layer().parent(&app), Some(parent_layer));
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
        let opacity = app
            .handle::<OpacityLayer>(layer.id())
            .expect("OpacityLayer");
        assert_eq!(opacity.alpha(&app), Some(128));
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

        let layer = tree.parent.as_object().layer(&app).expect("painted");
        let scene = layer.build_scene(&mut app, SceneBuilder::new());
        let ops = scene.ops().to_vec();
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

//! A box that shows a composition layer the host composites: Flutter's platform view render
//! object, over the `CompositionLayer` a producer feeds.
//!
//! It paints nothing of its own: it adds a [`EmbeddedLayer`] at its place in paint order, and
//! whoever composites the scene makes room for the composition layer there, so what the
//! framework paints above it stays and what it painted beneath is gone. It fills what it is
//! given: the layer's producer is told the size by whoever built this widget, not by the
//! compositor. It always needs compositing, as Flutter's platform views do, so a transform, a
//! clip or an opacity above it is a layer the composition layer is placed under rather than a
//! change to the canvas alone. The invariant: the layer added is the box, at the offset it is
//! painted at.

use std::sync::Arc;

use inset_embedder::{CompositionLayer, Offset, Size};
use inset_foundation::App;

use crate::box_::{BoxConstraints, RenderBox, RenderBoxData};
use crate::layer::{EmbeddedLayer, ErasedLayer};
use crate::object::{RenderHandle, RenderObject, RenderObjectData};
use crate::painting_context::PaintingContext;

/// Shows a composition layer in the box it is given.
pub struct RenderCompositionLayerView {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    composition_layer: Arc<dyn CompositionLayer>,
}

impl RenderCompositionLayerView {
    pub fn new(
        app: &mut App,
        composition_layer: Arc<dyn CompositionLayer>,
    ) -> RenderHandle<RenderCompositionLayerView> {
        RenderHandle::new_box(
            app,
            RenderCompositionLayerView {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                composition_layer,
            },
        )
    }

    /// The composition layer shown.
    pub fn composition_layer(self: RenderHandle<Self>, app: &App) -> &Arc<dyn CompositionLayer> {
        &self.get(app).composition_layer
    }

    /// Shows `composition_layer` instead, from the next paint.
    pub fn set_composition_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        composition_layer: Arc<dyn CompositionLayer>,
    ) {
        if Arc::ptr_eq(&self.get(app).composition_layer, &composition_layer) {
            return;
        }
        self.get_mut(app).composition_layer = composition_layer;
        self.mark_needs_paint(app);
    }
}

impl RenderObject for RenderCompositionLayerView {
    crate::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let size = self.constraints(app).biggest();
        self.set_size(app, size);
    }

    /// The composition layer placed at the box: the layer splits the picture there, so what is
    /// painted after it lies above the composition layer.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let composition_layer = Arc::clone(&self.get(app).composition_layer);
        let layer = EmbeddedLayer::new(app, composition_layer, offset & self.size(app));
        context.add_layer(app, layer.as_layer());
    }
}

impl RenderBox for RenderCompositionLayerView {
    crate::render_box_accessors!();

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.biggest()
    }

    /// A composition layer takes a hit anywhere in its box, so a listener around it hears the
    /// pointer.
    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::test_support::idle_layer;
    use inset_embedder::{Clip, Matrix4, Mutator, Part, Rect, SceneBuilder};
    use inset_foundation::{AppCell, Handle};

    use super::*;
    use crate::layer::{ContainerLayer, OffsetLayer};
    use crate::object::AnyRenderObject;
    use crate::pipeline_owner::PipelineOwner;

    /// Where a [`Parent`] paints its child.
    const CHILD_OFFSET: Offset = Offset::new(10.0, 5.0);

    /// A parent that lays its one child out at a fixed size and paints it as `painting` says.
    struct Parent {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        child: RenderHandle<RenderCompositionLayerView>,
        painting: ChildPainting,
    }

    /// How a [`Parent`] paints its child.
    #[derive(Clone, Copy)]
    enum ChildPainting {
        /// At [`CHILD_OFFSET`].
        AtOffset,
        /// At [`CHILD_OFFSET`], at half its size, clipped to the parent's top ten pixels.
        HalvedUnderAClip,
    }

    impl RenderObject for Parent {
        crate::render_object_accessors!();

        /// The root of a painted tree owns the root layer.
        fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
            true
        }

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let child = self.get(app).child;
            child.layout(app, BoxConstraints::tight(Size::new(30.0, 20.0)), false);
            self.set_size(app, Size::new(100.0, 100.0));
        }

        fn visit_children(
            self: RenderHandle<Self>,
            app: &App,
            visitor: &mut dyn FnMut(AnyRenderObject),
        ) {
            visitor(self.get(app).child.as_object());
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
        ) {
            let child = self.get(app).child.as_object();
            match self.get(app).painting {
                ChildPainting::AtOffset => context.paint_child(app, child, offset + CHILD_OFFSET),
                ChildPainting::HalvedUnderAClip => {
                    let needs_compositing = self.as_object().needs_compositing(app);
                    context.push_clip_rect(
                        app,
                        needs_compositing,
                        offset,
                        Rect::from_ltwh(0.0, 0.0, 100.0, 10.0),
                        |app, context, offset| {
                            context.push_transform(
                                app,
                                needs_compositing,
                                offset + CHILD_OFFSET,
                                Matrix4::scale(0.5, 0.5),
                                |app, context, offset| context.paint_child(app, child, offset),
                                None,
                            );
                        },
                        Clip::HardEdge,
                        None,
                    );
                }
            }
        }
    }

    impl RenderBox for Parent {
        crate::render_box_accessors!();
    }

    /// Lays out and paints a view of `composition_layer` under a parent that paints it as
    /// `painting` says, and answers the scene.
    fn scene_of_one_frame(
        app: &mut App,
        composition_layer: Arc<dyn CompositionLayer>,
        painting: ChildPainting,
    ) -> (inset_embedder::Scene, Handle<PipelineOwner>) {
        let owner = PipelineOwner::new(app, None, None);
        let child = RenderCompositionLayerView::new(app, composition_layer);
        let parent = RenderHandle::new_box(
            app,
            Parent {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child,
                painting,
            },
        );
        parent.adopt_child(app, child.as_object());
        owner.set_root_node(app, Some(parent.as_object()));
        parent.schedule_initial_layout(app);
        let root = OffsetLayer::new(app, Offset::ZERO);
        root.as_layer().attach(app, parent.as_object().id());
        parent
            .as_object()
            .schedule_initial_paint(app, root.as_container_layer());
        owner.flush_layout(app);
        owner.flush_compositing_bits(app);
        owner.flush_paint(app);
        let scene = root
            .as_container_layer()
            .build_scene(app, SceneBuilder::new());
        (scene, owner)
    }

    #[test]
    fn the_box_is_placed_and_nothing_is_painted() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let composition_layer = idle_layer();
        let (scene, _owner) = scene_of_one_frame(
            &mut app,
            Arc::clone(&composition_layer),
            ChildPainting::AtOffset,
        );
        let slices = inset_embedder::slices(&scene, |_| None);
        let [Part::CompositionLayer { layer, placement }] = slices.parts() else {
            panic!("the composition layer alone: {:?}", slices.parts());
        };
        assert!(Arc::ptr_eq(layer, &composition_layer));
        let placed = Rect::from_ltwh(10.0, 5.0, 30.0, 20.0);
        assert_eq!(placement.bounds(), placed);
        assert_eq!(placement.visible_bounds(), placed);
        assert!(!slices.paints());
    }

    /// A transform and a clip above a composition layer are layers of their own, as they are
    /// above Flutter's platform views, so the scene places the composition layer under both.
    #[test]
    fn a_composition_layer_is_placed_under_the_transform_and_the_clip_above_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (scene, _owner) =
            scene_of_one_frame(&mut app, idle_layer(), ChildPainting::HalvedUnderAClip);
        let slices = inset_embedder::slices(&scene, |_| None);
        let (_, placement) = slices.composition_layers().next().expect("placed");
        assert!(
            placement
                .mutators
                .contains(&Mutator::ClipRect(Rect::from_ltwh(0.0, 0.0, 100.0, 10.0)))
        );
        assert_eq!(placement.scale(), 0.5);
        assert_eq!(placement.bounds(), Rect::from_ltwh(10.0, 5.0, 15.0, 10.0));
        assert_eq!(
            placement.visible_bounds(),
            Rect::from_ltrb(10.0, 5.0, 25.0, 10.0)
        );
    }
}

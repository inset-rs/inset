//! Flutter counterpart: dart:ui `SceneBuilder`, as the framework's layers call it from
//! `add_to_scene`.
//!
//! It builds the engine's layer tree as `lib/ui/compositing/scene_builder.cc` does, with a stack
//! of the containers being built: a push starts one inside the last, `pop` builds the last and
//! adds it to the one beneath, and an add puts a leaf, or a retained container, in the last. It
//! records nothing and computes nothing; what the containers do is replayed by whoever walks the
//! tree. The invariant a later editor must keep: a node is built before it is shared, so `pop`
//! hands back a container no later call changes.

use std::fmt;
use std::sync::Arc;

use super::Scene;
use super::layer::{ContainerLayer, DisplayListLayer, Effect, EngineLayer, Layer, SurfaceLayer};
use crate::composition::Surface;
use crate::geometry::{Clip, Matrix4, Offset, RRect, Rect};
use crate::painting::{BlendMode, ImageFilter, Path, Picture};

/// Builds a [`Scene`] containing the given visuals.
///
/// A [`Scene`] can then be rendered using [`crate::View::present`].
///
/// To draw graphical operations onto a [`Scene`], first create a [`Canvas`](crate::Canvas),
/// record into it, and add the resulting [`Picture`] with [`add_picture`](Self::add_picture).
/// Each `push_*` starts a container that [`pop`](Self::pop) builds into an [`EngineLayer`];
/// unbalanced pushes are a bug the builder asserts on.
pub struct SceneBuilder {
    /// The containers being built, the scene's root first, as the engine's `layer_stack_`:
    /// every push starts one inside the last, and every pop builds the last into the one beneath.
    /// `build` builds the root.
    layer_stack: Vec<ContainerLayerBuilder>,
}

/// The [`ContainerLayer`] being built while its push is open: its effect, and the children added
/// so far.
struct ContainerLayerBuilder {
    effect: Effect,
    children: Vec<Layer>,
}

impl ContainerLayerBuilder {
    fn new(effect: Effect) -> ContainerLayerBuilder {
        ContainerLayerBuilder {
            effect,
            children: Vec::new(),
        }
    }

    fn build(self) -> ContainerLayer {
        ContainerLayer {
            effect: self.effect,
            children: self.children,
        }
    }
}

impl fmt::Debug for SceneBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SceneBuilder")
            .field("open_pushes", &(self.layer_stack.len() - 1))
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
            layer_stack: vec![ContainerLayerBuilder::new(Effect::None)],
        }
    }

    /// Pushes a transform operation onto the operation stack.
    ///
    /// The objects are transformed by the given matrix before rasterization.
    pub fn push_transform(&mut self, matrix4: &Matrix4) {
        self.push_layer(Effect::Transform(*matrix4));
    }

    /// Pushes an offset operation onto the operation stack.
    ///
    /// This is equivalent to [`push_transform`](Self::push_transform) with a matrix with only
    /// translation.
    pub fn push_offset(&mut self, dx: f64, dy: f64) {
        self.push_layer(Effect::Transform(Matrix4::translation(
            dx as f32, dy as f32,
        )));
    }

    /// Pushes a rectangular clip operation onto the operation stack.
    ///
    /// Rasterization outside the given rectangle is discarded.
    pub fn push_clip_rect(&mut self, rect: Rect, clip_behavior: Clip) {
        self.push_layer(Effect::ClipRect {
            rect,
            clip_behavior,
        });
    }

    /// Pushes a rounded-rectangular clip operation onto the operation stack.
    ///
    /// Rasterization outside the given rounded rectangle is discarded.
    pub fn push_clip_rrect(&mut self, rrect: RRect, clip_behavior: Clip) {
        self.push_layer(Effect::ClipRRect {
            rrect,
            clip_behavior,
        });
    }

    /// Pushes a path clip operation onto the operation stack.
    ///
    /// Rasterization outside the given path is discarded.
    pub fn push_clip_path(&mut self, path: &Arc<Path>, clip_behavior: Clip) {
        self.push_layer(Effect::ClipPath {
            path: Arc::clone(path),
            clip_behavior,
        });
    }

    /// Pushes an opacity operation onto the operation stack.
    ///
    /// The given alpha value is blended into the alpha value of the objects' rasterization. An
    /// alpha value of 0 makes the objects entirely invisible. An alpha value of 255 has no
    /// effect (i.e., the objects retain the current opacity).
    pub fn push_opacity(&mut self, alpha: i32, offset: Offset) {
        self.push_layer(Effect::Opacity {
            alpha: alpha.clamp(0, 255) as u8,
            offset,
        });
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
        self.push_layer(Effect::BackdropFilter {
            filter: filter.clone(),
            blend_mode,
            backdrop_id,
        });
    }

    /// Ends the effect of the most recently pushed operation.
    ///
    /// The container built is the [`EngineLayer`] returned, a child of the one beneath it: the
    /// layer that pushed keeps it and hands it to [`add_retained`](Self::add_retained) while its
    /// subtree stays unchanged. dart:ui returns the engine layer from the push; a container here
    /// is built, and shared, at its pop.
    ///
    /// # Panics
    ///
    /// If nothing is pushed.
    pub fn pop(&mut self) -> EngineLayer {
        assert!(
            self.layer_stack.len() > 1,
            "SceneBuilder::pop without a push"
        );
        let built = self.layer_stack.pop().expect("a push is open").build();
        let layer = Arc::new(built);
        self.add_layer(Layer::Container(Arc::clone(&layer)));
        layer
    }

    /// Places a native surface the host composites, at `rect` in the coordinates of the container
    /// being built: Flutter's `addPlatformView`. It draws nothing; whoever composites the scene
    /// makes room for it at its place in paint order.
    pub fn add_surface(&mut self, surface: Arc<dyn Surface>, rect: Rect) {
        self.add_layer(Layer::Surface(SurfaceLayer { surface, rect }));
    }

    /// Adds a retained [`EngineLayer`] subtree from a previous frame.
    ///
    /// All the engine layers that are in the subtree of the retained layer will be automatically
    /// appended to the current engine layer tree: the same node, in the container being built.
    pub fn add_retained(&mut self, retained_layer: &EngineLayer) {
        self.add_layer(Layer::Container(Arc::clone(retained_layer)));
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
        self.add_layer(Layer::DisplayList(DisplayListLayer {
            offset,
            picture: Arc::clone(picture),
        }));
    }

    /// Finishes building the scene.
    ///
    /// # Panics
    ///
    /// In debug builds, if a pushed operation was not popped.
    pub fn build(mut self) -> Scene {
        debug_assert!(
            self.layer_stack.len() == 1,
            "SceneBuilder::build with an open push"
        );
        while self.layer_stack.len() > 1 {
            self.pop();
        }
        let root = self.layer_stack.pop().expect("the root is being built");
        Scene { root: root.build() }
    }

    /// Starts building a container inside the last: the engine's `PushLayer`.
    fn push_layer(&mut self, effect: Effect) {
        self.layer_stack.push(ContainerLayerBuilder::new(effect));
    }

    /// Adds `layer` to the container being built last: the engine's `AddLayer`.
    fn add_layer(&mut self, layer: Layer) {
        self.layer_stack
            .last_mut()
            .expect("the root is built only by build")
            .children
            .push(layer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Canvas, Paint};

    fn square() -> Arc<Picture> {
        let mut canvas = Canvas::new();
        canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), &Paint::default());
        Arc::new(canvas.build())
    }

    fn container(layer: &Layer) -> &EngineLayer {
        match layer {
            Layer::Container(container) => container,
            other => panic!("not a container: {other:?}"),
        }
    }

    #[test]
    fn a_popped_container_is_a_node_of_its_parent() {
        let picture = square();
        let mut builder = SceneBuilder::new();
        builder.push_opacity(128, Offset::new(1.0, 2.0));
        builder.push_transform(&Matrix4::scale(2.0, 2.0));
        builder.add_picture(Offset::new(3.0, 4.0), &picture, false, false);
        let inner = builder.pop();
        let outer = builder.pop();
        let scene = builder.build();

        assert_eq!(scene.root.effect, Effect::None);
        let [only] = scene.root.children.as_slice() else {
            panic!("one child: {:?}", scene.root.children);
        };
        assert!(Arc::ptr_eq(container(only), &outer));
        assert_eq!(
            outer.effect,
            Effect::Opacity {
                alpha: 128,
                offset: Offset::new(1.0, 2.0)
            }
        );
        assert!(Arc::ptr_eq(container(&outer.children[0]), &inner));
        assert_eq!(inner.effect, Effect::Transform(Matrix4::scale(2.0, 2.0)));
        assert_eq!(
            inner.children,
            [Layer::DisplayList(DisplayListLayer {
                offset: Offset::new(3.0, 4.0),
                picture,
            })]
        );
    }

    #[test]
    fn a_retained_container_is_the_same_node_in_the_next_scene() {
        let mut builder = SceneBuilder::new();
        builder.push_clip_rect(Rect::from_ltwh(0.0, 0.0, 5.0, 5.0), Clip::HardEdge);
        builder.add_picture(Offset::ZERO, &square(), false, false);
        let retained = builder.pop();
        let _first = builder.build();

        let mut builder = SceneBuilder::new();
        builder.push_offset(10.0, 0.0);
        builder.add_retained(&retained);
        builder.pop();
        let next = builder.build();
        let moved = container(&next.root.children[0]);
        assert!(Arc::ptr_eq(container(&moved.children[0]), &retained));
    }

    #[test]
    fn an_opacity_is_kept_between_none_and_all() {
        let mut builder = SceneBuilder::new();
        builder.push_opacity(300, Offset::ZERO);
        let all = builder.pop();
        builder.push_opacity(-1, Offset::ZERO);
        let none = builder.pop();
        let alpha = |layer: &EngineLayer| match layer.effect {
            Effect::Opacity { alpha, .. } => alpha,
            _ => unreachable!(),
        };
        assert_eq!((alpha(&all), alpha(&none)), (255, 0));
    }

    #[test]
    fn a_surface_is_a_leaf_at_its_place_in_paint_order() {
        let surface = super::super::idle_surface();
        let rect = Rect::from_ltwh(1.0, 2.0, 3.0, 4.0);
        let mut builder = SceneBuilder::new();
        builder.add_picture(Offset::ZERO, &square(), false, false);
        builder.add_surface(Arc::clone(&surface), rect);
        let scene = builder.build();
        assert!(matches!(scene.root.children[0], Layer::DisplayList(_)));
        assert_eq!(
            scene.root.children[1],
            Layer::Surface(SurfaceLayer { surface, rect })
        );
    }

    #[test]
    fn a_scene_without_layers_has_an_empty_root() {
        let scene = SceneBuilder::new().build();
        assert!(scene.root.children.is_empty());
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

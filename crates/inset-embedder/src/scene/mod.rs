//! Flutter counterpart: dart:ui `Scene` and `SceneBuilder`, and the engine's layer tree they
//! build (`flow/layers`).
//!
//! A frame's scene keeps its layers. The framework's layers build a tree of containers, pictures
//! and native surfaces through the builder (`builder.rs`, `layer.rs`), and a host gets that tree
//! itself in [`View::present`](crate::View::present). What a host composites, the drawing between
//! surfaces and each surface with the transforms, clips and opacity above it (`placement.rs`), is
//! what [`slices`] makes of the tree (`slices.rs`): a utility every host calls today, which a
//! host needing a walk of its own does without. valo only draws: what a container does to its
//! children is replayed into valo's lists by the walk, and nothing of compositing enters them.
//!
//! The invariant a later editor must keep: the tree is the rawest record of a frame, built
//! without computing anything, and every answer a host needs is worked out from it in one place.

mod builder;
mod layer;
mod placement;
mod slices;

pub use builder::SceneBuilder;
pub use layer::{ContainerLayer, DisplayListLayer, Effect, EngineLayer, Layer, SurfaceLayer};
pub use placement::{Mutator, SurfacePlacement};
pub use slices::{Part, Slices, slices};

/// Flutter `Scene`: what [`SceneBuilder::build`] produces and [`crate::View::present`] takes, the
/// root of one frame's layer tree, flow's `LayerTree`. The view knows its own size.
///
/// Two scenes are equal when they show the same: what a host asks to tell a scene equal to the
/// one it shows from one worth drawing. The comparison takes the time of the containers the frame
/// built afresh, since a retained container compares by pointer and a picture by valo's identity
/// first.
#[derive(Debug, PartialEq)]
pub struct Scene {
    /// The root container, whose effect is [`Effect::None`].
    pub root: ContainerLayer,
}

/// A surface no producer feeds, for the tests of this module. The crates above the embedder use
/// `test_support`'s, which this crate's own tests are built without.
#[cfg(test)]
fn idle_surface() -> std::sync::Arc<dyn crate::Surface> {
    struct IdleSurface;

    impl crate::Surface for IdleSurface {
        fn present(&self, _frame: crate::SurfaceFrame, _released: Box<dyn FnOnce() + Send + Sync>) {
        }
    }

    impl crate::SurfaceParent for IdleSurface {
        fn open_child(&self, _opaque: bool) -> std::sync::Arc<dyn crate::Surface> {
            idle_surface()
        }
    }

    std::sync::Arc::new(IdleSurface)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{Canvas, Offset, Paint, Rect};

    fn square() -> Arc<crate::Picture> {
        let mut canvas = Canvas::new();
        canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), &Paint::default());
        Arc::new(canvas.build())
    }

    /// A scene crosses threads, as a host drawing on a thread of its own hands it there, wherever
    /// valo's pictures do: not on the web, whose GPU objects stay on their thread.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_scene_crosses_threads() {
        fn sendable<T: Send + Sync>() {}
        sendable::<Scene>();
    }

    #[test]
    fn a_scene_that_retains_a_container_equals_one_that_built_the_same() {
        let picture = square();
        let mut builder = SceneBuilder::new();
        builder.push_offset(1.0, 2.0);
        builder.add_picture(Offset::ZERO, &picture, false, false);
        let retained = builder.pop();
        let first = builder.build();

        let mut builder = SceneBuilder::new();
        builder.add_retained(&retained);
        assert_eq!(builder.build(), first);

        let mut builder = SceneBuilder::new();
        builder.push_offset(1.0, 2.0);
        builder.add_picture(Offset::ZERO, &square(), false, false);
        builder.pop();
        assert_eq!(
            builder.build(),
            first,
            "an equal picture in a new container"
        );

        let mut builder = SceneBuilder::new();
        builder.push_offset(1.0, 3.0);
        builder.add_picture(Offset::ZERO, &picture, false, false);
        builder.pop();
        assert_ne!(builder.build(), first, "the same picture elsewhere");
    }
}

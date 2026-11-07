//! The layer a view shows into on macOS: a composition layer of the view's own, whose content is the
//! `CAMetalLayer` the view draws into, shown as a player's layer is (`show_system_layer`), with the
//! composition layers the view's scenes place beneath it, in the composition layer's content. wgpu
//! makes the view's surface from that `CAMetalLayer`, not from a window's handle, so a view draws
//! the same way wherever its composition layer is placed. Flutter's macOS embedder arranges its own
//! layers in the layer of its view in the same way (`FlutterSurfaceManager.mm`), and so does
//! Chromium.
//!
//! Whoever holds the view places the composition layer, and the view sets on it what its metrics
//! say of its drawing: the `CAMetalLayer`'s scale here, and its drawable's size through its
//! surface, which wgpu configures. A window's code places it in the layer of winit's view, filling
//! it, and keeps it filling that layer as its bounds and scale change (`os/macos/composition.rs`).
//! The `CAMetalLayer` fills the composition layer, its drawable anchored at the visual top-left as
//! the composition layer's own buffers are (`mod.rs`): a drawable of another size, one a resize's
//! frame did not replace in time, as during a window's zoom, shows at its own size rather than
//! stretched, as Chromium shows its frames (`display_ca_layer_tree.mm`, `kCAGravityTopLeft`).
//!
//! The invariant: the `CAMetalLayer` is the topmost sublayer of the composition layer's content,
//! above every composition layer the view's scenes place there, as `waft-embedder`'s README asks
//! of a host (its composition layers beneath the picture); and wgpu sets its device, pixel format,
//! colour space, opacity, drawable count and size, and display sync at each configuration, as it
//! does on a layer it makes itself.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use objc2::rc::Retained;
use objc2_core_graphics::CGColor;
use objc2_quartz_core::CAMetalLayer;
use waft_embedder::{CompositionLayer, Placement, ViewId};

use super::placed::PlacedLayers;
use super::{CoreAnimationLayer, Placer, adopted_by, in_transaction, own_layers};
use crate::gpu::Gpu;
use crate::os::capabilities::ViewLayer;

/// A view's composition layer, the `CAMetalLayer` the view draws into as its content, and the
/// composition layers the view's last scene placed beneath it.
pub(crate) struct CoreAnimationViewLayer {
    /// What the view's holder places.
    layer: Arc<CoreAnimationLayer>,
    /// What the view draws into: the layer of wgpu's surface.
    drawing: Retained<CAMetalLayer>,
    /// The composition layers the view's last scene placed, beneath `drawing`.
    placed: RefCell<PlacedLayers>,
    /// The view's device pixel ratio, at which its scenes' composition layers are laid out;
    /// `None` until the view is given its metrics.
    scale: Cell<Option<f64>>,
}

impl CoreAnimationViewLayer {
    /// The layer a view shows into on `layer`, a composition layer this host's compositor made:
    /// its content is a `CAMetalLayer` nothing has drawn into yet. `None` for a layer another host
    /// made. Main thread.
    pub(crate) fn on(layer: &Arc<dyn CompositionLayer>) -> Option<CoreAnimationViewLayer> {
        let layer = CoreAnimationLayer::downcast_from(layer)?;
        let drawing = CAMetalLayer::new();
        // Bottom-left is the visual top-left in a flipped view's layer, as for the buffer layer.
        // SAFETY: a constant Core Animation exports for the life of the process.
        drawing.setContentsGravity(unsafe { objc2_quartz_core::kCAGravityBottomLeft });
        layer.show_system_layer(&drawing);
        Some(CoreAnimationViewLayer {
            layer,
            drawing,
            placed: RefCell::new(PlacedLayers::new()),
            scale: Cell::new(None),
        })
    }

    /// Whether the composition layer was made opaque (`Compositor::make_layer`).
    pub(crate) fn is_opaque(&self) -> bool {
        self.layer.opaque
    }
}

impl ViewLayer for CoreAnimationViewLayer {
    /// The composition layer is Core Animation's, which shows it and the composition layers in it
    /// over its background whether or not anything is drawn above them.
    const SHOWS_FRAMES_UNDRAWN: bool = true;

    fn set_scale(&self, scale: f64) {
        self.scale.set(Some(scale));
        in_transaction(|| self.drawing.setContentsScale(scale));
    }

    fn make_surface(
        &self,
        gpu: &Gpu,
        size: [u32; 2],
        alpha: valo::SurfaceAlpha,
    ) -> Result<valo::Surface, String> {
        let layer = Retained::as_ptr(&self.drawing).cast_mut().cast();
        // SAFETY: a live `CAMetalLayer`, which wgpu retains for as long as the surface lives.
        unsafe {
            valo::Surface::new_unsafe_with_options(
                &gpu.instance,
                &gpu.adapter,
                &gpu.device,
                wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer),
                size,
                valo::SurfaceOptions::default().with_alpha(alpha),
            )
        }
        .map_err(|error| error.to_string())
    }

    /// Places the layers of `view`'s scene in the composition layer's content, beneath the
    /// drawing, in paint order, and lets go of the ones the scene no longer places.
    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    ) {
        let layers = adopted_by(own_layers(placements), Placer::View(view));
        if layers.is_empty() && self.placed.borrow().is_empty() {
            return;
        }
        in_transaction(|| {
            self.placed
                .borrow_mut()
                .replace(layers, &self.layer.content_layer.0, self.scale.get());
        });
    }

    /// The composition layer's background, beneath the composition layers in it.
    fn show_background(&self, color: valo::Color) {
        let color = CGColor::new_srgb(
            color.r.into(),
            color.g.into(),
            color.b.into(),
            color.a.into(),
        );
        in_transaction(|| self.layer.content_layer.0.setBackgroundColor(Some(&color)));
    }
}

impl Drop for CoreAnimationViewLayer {
    /// The view is removed: its drawing and its background leave the composition layer, which a
    /// holder may still place, and the composition layers its scenes placed are let go.
    fn drop(&mut self) {
        in_transaction(|| {
            self.layer.take_down_system_layer();
            self.layer.content_layer.0.setBackgroundColor(None);
        });
    }
}

#[cfg(test)]
mod tests {
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use objc2_quartz_core::CALayer;
    use waft_embedder::{Compositor, Matrix4, Mutator, Rect};

    use super::super::{CoreAnimationCompositor, place_filling};
    use super::*;

    /// A window 640 by 480 points at twice as many pixels: the composition layer and the drawing
    /// fill the layer of its view, the drawing at the view's scale.
    #[test]
    fn the_drawing_fills_the_window_at_its_scale() {
        let view_layer = flipped_window_layer(640.0, 480.0);
        let (made, layer) = made_view_layer();
        place_filling(&view_layer, &made, 2.0);
        layer.set_scale(2.0);

        let whole = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 480.0));
        assert_eq!(frame_in(&layer.layer.clip_layer.0, &view_layer), whole);
        assert_eq!(frame_in(&layer.drawing, &view_layer), whole);
        assert_eq!(layer.drawing.contentsScale(), 2.0);

        view_layer.setBounds(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(800.5, 600.0),
        ));
        place_filling(&view_layer, &made, 2.0);
        assert_eq!(
            frame_in(&layer.drawing, &view_layer).size,
            NSSize::new(800.5, 600.0),
            "placed again, it follows the window's bounds exactly"
        );
        assert_eq!(
            unsafe { view_layer.sublayers() }.unwrap().count(),
            1,
            "and stays where it is"
        );
    }

    /// The drawing's contents are flipped as those of a layer directly in winit's flipped view
    /// are, so a drawable shows upright as it would there.
    #[test]
    fn the_drawing_is_flipped_as_a_layer_directly_in_the_view_is() {
        let view_layer = flipped_window_layer(640.0, 480.0);
        let (made, layer) = made_view_layer();
        place_filling(&view_layer, &made, 2.0);
        let directly_in_view = CALayer::new();
        view_layer.addSublayer(&directly_in_view);
        assert_eq!(
            layer.drawing.contentsAreFlipped(),
            directly_in_view.contentsAreFlipped()
        );
    }

    /// A drawable of another size than the layer, as during a window's zoom, shows at its own size
    /// from the visual top-left, as the composition layer's buffers do, not stretched over it.
    #[test]
    fn a_drawable_of_another_size_shows_at_its_own_size() {
        let (_made, layer) = made_view_layer();
        // SAFETY: a constant Core Animation exports for the life of the process.
        let anchored = unsafe { objc2_quartz_core::kCAGravityBottomLeft };
        assert_eq!(&*layer.drawing.contentsGravity(), anchored);
    }

    /// A video a scene places lies where it lay in the layer of the window's view, beneath the
    /// drawing.
    #[test]
    fn a_scenes_layer_lies_beneath_the_drawing_where_it_lay_in_the_window() {
        let view_layer = flipped_window_layer(640.0, 480.0);
        let (made, layer) = made_view_layer();
        place_filling(&view_layer, &made, 2.0);
        layer.set_scale(2.0);
        let video = CoreAnimationCompositor.make_layer(true);
        let placement = Placement {
            rect: Rect::from_ltwh(20.0, 40.0, 320.0, 180.0),
            mutators: vec![
                Mutator::Transform(Matrix4::scale(2.0, 2.0)),
                Mutator::Transform(Matrix4::translation(10.0, 5.0)),
            ],
        };
        layer.place([(&video, &placement)], ViewId(1));

        let in_window = CoreAnimationLayer::new(true);
        in_window.show_in(&view_layer, 0, &placement, 2.0);
        let video_layers = CoreAnimationLayer::downcast_from(&video).unwrap();
        assert_eq!(
            frame_in(&video_layers.buffer_layer.0, &view_layer),
            frame_in(&in_window.buffer_layer.0, &view_layer)
        );
        let content = unsafe { layer.layer.content_layer.0.sublayers() }.unwrap();
        assert!(std::ptr::eq(
            &*content.objectAtIndex(0),
            &*video_layers.clip_layer.0
        ));
        assert!(
            std::ptr::eq(&*content.lastObject().unwrap(), &**layer.drawing),
            "the drawing is above the scene's layers"
        );
    }

    /// A view removed takes its drawing and its background out of a composition layer its holder
    /// still holds.
    #[test]
    fn a_removed_view_takes_its_drawing_down() {
        let (_made, layer) = made_view_layer();
        layer.show_background(valo::Color::WHITE);
        let drawing = layer.drawing.clone();
        let content = layer.layer.content_layer.0.clone();
        assert!(content.backgroundColor().is_some());
        drop(layer);
        assert!(drawing.superlayer().is_none());
        assert!(content.backgroundColor().is_none());
    }

    /// A composition layer its holder keeps, and the view's layer on it.
    fn made_view_layer() -> (Arc<dyn CompositionLayer>, CoreAnimationViewLayer) {
        let made = CoreAnimationCompositor.make_layer(false);
        let layer = CoreAnimationViewLayer::on(&made).unwrap();
        (made, layer)
    }

    /// The layer of a view winit makes on macOS, `width` by `height` points: flipped, so laid out
    /// from the top-left.
    fn flipped_window_layer(width: f64, height: f64) -> Retained<CALayer> {
        let layer = CALayer::new();
        layer.setGeometryFlipped(true);
        layer.setBounds(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(width, height),
        ));
        layer
    }

    /// Where `layer`'s bounds lie in `ancestor`'s coordinates.
    fn frame_in(layer: &CALayer, ancestor: &CALayer) -> NSRect {
        layer.convertRect_toLayer(layer.bounds(), Some(ancestor))
    }
}

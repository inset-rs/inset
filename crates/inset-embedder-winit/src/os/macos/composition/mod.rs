//! Native surfaces as Core Animation layers beneath a window's content.
//!
//! A surface is a `CALayer` whose contents are the `IOSurface` its producer presented last,
//! inside a layer of its own that cuts it to what the scene shows of it. It joins the layer
//! tree of the winit view whose scene first places it, as a sublayer below the `CAMetalLayer`
//! wgpu draws the framework into (raw-window-metal makes that one a sublayer too), so the clear
//! rectangle the framework paints where the surface goes shows it through. A buffer shows at
//! the scale the scene places the surface at: whole, and smaller, under a transform that shrinks
//! it. `docs/composition.md` has the design, and the experiment that settled each choice here.
//!
//! The module holds the compositor and the surface; `buffers` what a surface was presented and
//! when each goes back to its producer, `window_surfaces` placing a scene's surfaces in a window,
//! and `window_vsync` a window's share of its screen's display link.
//!
//! Three things touch a surface. Its producer presents from its own thread, in an explicit
//! transaction. The main thread places it with the window's frame. The display link of the screen
//! the window is on, shared with the framework's frames and that screen's other windows
//! (`display_links.rs`), on the main thread, releases the buffers the compositor has finished with
//! and calls the producers waiting for a vsync, while the window is visible (`visibility.rs`); a
//! producer that asks while the window's ticks are paused has the main queue restart them, so
//! nothing of this reaches the embedder's event loop. Out of sight, the window's ticks stop, and
//! AppKit's notice that it may be seen again restarts them. The invariant a later editor must keep:
//! a buffer's release callback runs only once `IOSurfaceIsInUse` answers false for it, or when its
//! surface is gone.

mod buffers;
mod window_surfaces;
mod window_vsync;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use inset_embedder::{
    Buffer, Compositor, Offset, Rect, Surface, SurfacePlacement, ViewId, VsyncObserver,
};
use objc2::rc::Retained;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CALayer, CATransaction};

use super::AppKit;
use crate::os::NativeComposition;
use buffers::{CrossThreadBuffer, PresentedBuffers, notify_released};
use window_vsync::PlacedSurfaces;

use window_surfaces::CoreAnimationWindowSurfaces;

const HELD_BRIEFLY: &str = "a surface's state is never held across a panic";

impl NativeComposition for AppKit {
    type WindowSurfaces = CoreAnimationWindowSurfaces;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        Some(Arc::new(CoreAnimationCompositor))
    }
}

/// The compositor of a macOS embedder. Every surface is a layer of its own, so it keeps nothing.
struct CoreAnimationCompositor;

impl Compositor for CoreAnimationCompositor {
    fn open_surface(&self, view: ViewId, opaque: bool) -> Arc<dyn Surface> {
        Arc::new(CoreAnimationSurface::new(view, opaque))
    }
}

/// A surface: its layers, the buffers presented to it and not yet released, and the producers
/// waiting for a vsync.
pub(crate) struct CoreAnimationSurface {
    /// The view whose scenes may place the surface.
    view: ViewId,
    /// What a scene shows of the surface: the layer that joins the view's tree, masking
    /// [`layer`](Self::layer) to its bounds.
    clip_layer: CrossThreadLayer,
    /// The buffer's layer, inside the clip layer.
    layer: CrossThreadLayer,
    buffers: Mutex<PresentedBuffers<CrossThreadBuffer>>,
    vsync_observers: Mutex<Vec<Weak<dyn VsyncObserver>>>,
    /// The window the surface was placed in, once a scene has placed it.
    window: Mutex<Weak<PlacedSurfaces>>,
    /// Whether a placement in another view's scene was reported already.
    misplacement_reported: AtomicBool,
}

impl CoreAnimationSurface {
    fn new(view: ViewId, opaque: bool) -> CoreAnimationSurface {
        let layer = CALayer::new();
        layer.setOpaque(opaque);
        // winit's view is flipped: a sublayer's frame is laid out from the top-left, but its
        // contents gravity is not flipped with it, so bottom-left is the visual top-left. A
        // buffer smaller than the layer, drawn before a resize reached the producer, shows at
        // its own size from there rather than stretched.
        // SAFETY: a constant Core Animation exports for the life of the process.
        layer.setContentsGravity(unsafe { objc2_quartz_core::kCAGravityBottomLeft });
        let clip_layer = CALayer::new();
        clip_layer.setMasksToBounds(true);
        clip_layer.addSublayer(&layer);
        clip_layer.setHidden(true);
        CoreAnimationSurface {
            view,
            clip_layer: CrossThreadLayer(clip_layer),
            layer: CrossThreadLayer(layer),
            buffers: Mutex::new(PresentedBuffers::new()),
            vsync_observers: Mutex::new(Vec::new()),
            window: Mutex::new(Weak::new()),
            misplacement_reported: AtomicBool::new(false),
        }
    }

    /// The embedder's own surface behind a placed one; `None` for a surface no compositor of
    /// this embedder opened.
    fn downcast_from(surface: &Arc<dyn Surface>) -> Option<Arc<CoreAnimationSurface>> {
        let surface = Arc::clone(surface);
        let any: Arc<dyn std::any::Any + Send + Sync> = surface;
        any.downcast().ok()
    }

    /// Whether a scene of `view` may place the surface, reporting once when it may not.
    fn may_be_placed_in(&self, view: ViewId) -> bool {
        if view == self.view {
            return true;
        }
        if !self.misplacement_reported.swap(true, Ordering::Relaxed) {
            eprintln!(
                "inset: a surface opened for view {:?} was placed in view {:?}; it is not shown",
                self.view, view
            );
        }
        false
    }

    /// Puts the clip layer at `index` among `parent`'s sublayers and shows the surface where
    /// `placement` says, in a view `window_scale` of whose physical pixels make a point. Main
    /// thread, inside the caller's transaction.
    fn place(&self, parent: &CALayer, index: u32, placement: &SurfacePlacement, window_scale: f64) {
        let Some(contents_scale) = contents_scale_of(placement.scale, window_scale) else {
            self.hide();
            return;
        };
        let clip_layer = &self.clip_layer.0;
        if !is_sublayer_at(parent, clip_layer, index) {
            parent.insertSublayer_atIndex(clip_layer, index);
        }
        clip_layer.setFrame(layer_frame_of(placement.clip, window_scale));
        let within_clip = Offset::new(-placement.clip.left, -placement.clip.top);
        let layer = &self.layer.0;
        layer.setFrame(layer_frame_of(
            placement.rect.shift(within_clip),
            window_scale,
        ));
        layer.setContentsScale(contents_scale);
        clip_layer.setHidden(false);
    }

    fn hide(&self) {
        self.clip_layer.0.setHidden(true);
    }

    /// From now on the surface's vsyncs come through `window`, the surfaces of the window a
    /// scene placed it in.
    fn attach_to_window(&self, window: Weak<PlacedSurfaces>) {
        *self.window.lock().expect(HELD_BRIEFLY) = window;
    }

    fn is_waiting_for_vsync(&self) -> bool {
        !self.vsync_observers.lock().expect(HELD_BRIEFLY).is_empty()
    }

    fn take_vsync_observers(&self) -> Vec<Weak<dyn VsyncObserver>> {
        std::mem::take(&mut *self.vsync_observers.lock().expect(HELD_BRIEFLY))
    }

    /// Calls the release callback of every replaced buffer the compositor has finished with.
    fn release_unused_buffers(&self) {
        let released = self
            .buffers
            .lock()
            .expect(HELD_BRIEFLY)
            .release_unused(|buffer| buffer.0.is_in_use());
        notify_released(released);
    }
}

impl Surface for CoreAnimationSurface {
    fn present(&self, buffer: Buffer, released: Box<dyn FnOnce() + Send + Sync>) {
        let buffer = CrossThreadBuffer(buffer);
        in_transaction(|| buffer.set_as_contents_of(&self.layer.0));
        self.buffers
            .lock()
            .expect(HELD_BRIEFLY)
            .present(buffer, released);
        self.release_unused_buffers();
    }

    fn next_vsync(&self, observer: Weak<dyn VsyncObserver>) {
        let mut observers = self.vsync_observers.lock().expect(HELD_BRIEFLY);
        if !observers.iter().any(|known| Weak::ptr_eq(known, &observer)) {
            observers.push(observer);
        }
        // Released before the window's lock is taken, which a vsync takes around this one.
        drop(observers);
        // A surface no scene has placed yet has no window to take a vsync from; its first
        // placement starts the window's link for the observers waiting here.
        let window = self.window.lock().expect(HELD_BRIEFLY).upgrade();
        if let Some(window) = window {
            window.request_vsync();
        }
    }
}

impl Drop for CoreAnimationSurface {
    /// The layers leave their window, and every buffer goes back to the producer.
    fn drop(&mut self) {
        in_transaction(|| self.clip_layer.0.removeFromSuperlayer());
        let released = self.buffers.get_mut().expect(HELD_BRIEFLY).release_all();
        notify_released(released);
    }
}

/// Whether `layer` is `parent`'s sublayer at `index` already.
fn is_sublayer_at(parent: &CALayer, layer: &CALayer, index: u32) -> bool {
    // SAFETY: reading a layer's sublayers on the main thread, where they are changed.
    let Some(sublayers) = (unsafe { parent.sublayers() }) else {
        return false;
    };
    (index as usize) < sublayers.count()
        && std::ptr::eq(&*sublayers.objectAtIndex(index as usize), layer)
}

/// `rect`, in the view's physical pixels, as a layer's frame: in points, `scale` pixels each.
fn layer_frame_of(rect: Rect, scale: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.left / scale, rect.top / scale),
        NSSize::new(rect.width() / scale, rect.height() / scale),
    )
}

/// How many of a buffer's pixels make one of its layer's points, for a surface placed at
/// `placement_scale` in a window `window_scale` pixels to the point: a producer draws at the
/// window's scale, and the transforms above the surface shrink or grow what it drew by
/// `placement_scale / window_scale`. `None` for a surface shrunk to nothing.
fn contents_scale_of(placement_scale: f64, window_scale: f64) -> Option<f64> {
    (placement_scale > 0.0 && placement_scale.is_finite())
        .then(|| window_scale * window_scale / placement_scale)
}

/// Runs `change` in a transaction of its own with implicit animations off: a layer changed
/// off the main thread must be in one, and a layer the framework moves must not animate.
fn in_transaction(change: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    change();
    CATransaction::commit();
}

/// A `CALayer` handed between threads.
struct CrossThreadLayer(Retained<CALayer>);

// SAFETY: Core Animation guards its own state, and this module changes a layer off the main
// thread only inside an explicit transaction, as Chromium and WebKit do; the binding cannot
// say so. Measured safe for contents replacement in `2026-09-22-layer-under-a-hole`, in the
// `experiments` folder beside this repository.
unsafe impl Send for CrossThreadLayer {}
unsafe impl Sync for CrossThreadLayer {}

#[cfg(test)]
mod tests {
    use objc2_foundation::{NSPoint, NSSize};

    use super::*;

    #[test]
    fn a_placement_in_physical_pixels_is_a_frame_in_points() {
        let frame = layer_frame_of(Rect::from_ltwh(240.0, 180.0, 800.0, 600.0), 2.0);
        assert_eq!(frame.origin, NSPoint::new(120.0, 90.0));
        assert_eq!(frame.size, NSSize::new(400.0, 300.0));
    }

    #[test]
    fn a_buffer_shows_at_the_scale_its_surface_is_placed_at() {
        assert_eq!(
            contents_scale_of(2.0, 2.0),
            Some(2.0),
            "unscaled, a pixel of the buffer is a pixel of the display"
        );
        assert_eq!(
            contents_scale_of(1.0, 2.0),
            Some(4.0),
            "at half size, four of the buffer's pixels make a point"
        );
        assert_eq!(contents_scale_of(0.0, 2.0), None);
    }

    /// A card: a page 390 by 844 points, at half its size, of which a square in the middle
    /// shows. In a flipped view, as winit's is, the buffer's layer lies where the placement
    /// says whatever the clip layer between them, and its contents are flipped as a layer
    /// directly in the view would be, which is what makes bottom-left gravity the visual
    /// top-left.
    #[test]
    fn a_placed_surface_lies_where_its_placement_says_inside_its_clip() {
        let view_layer = CALayer::new();
        view_layer.setGeometryFlipped(true);
        let surface = CoreAnimationSurface::new(ViewId(0), true);
        let placement = SurfacePlacement {
            surface: idle_surface_for_placement(),
            rect: Rect::from_ltwh(100.0, 200.0, 390.0, 844.0),
            scale: 1.0,
            clip: Rect::from_ltwh(100.0, 427.0, 390.0, 390.0),
        };
        surface.place(&view_layer, 0, &placement, 2.0);

        let layer = &surface.layer.0;
        let placed = layer.convertRect_toLayer(layer.bounds(), Some(&view_layer));
        assert_eq!(placed.origin, NSPoint::new(50.0, 100.0));
        assert_eq!(placed.size, NSSize::new(195.0, 422.0));
        let clip_layer = &surface.clip_layer.0;
        assert_eq!(clip_layer.frame().origin, NSPoint::new(50.0, 213.5));
        assert!(clip_layer.masksToBounds() && !clip_layer.isHidden());
        assert_eq!(layer.contentsScale(), 4.0);

        let directly_in_view = CALayer::new();
        view_layer.addSublayer(&directly_in_view);
        assert_eq!(
            layer.contentsAreFlipped(),
            directly_in_view.contentsAreFlipped()
        );
    }

    /// A placement's own surface, which [`CoreAnimationSurface::place`] does not read.
    fn idle_surface_for_placement() -> Arc<dyn Surface> {
        Arc::new(CoreAnimationSurface::new(ViewId(0), true))
    }
}

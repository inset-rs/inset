//! Native surfaces as Core Animation layers beneath a window's content, on macOS and iOS alike.
//!
//! A surface is three layers. Its clip layer cuts it to what the placement shows of it. Inside, its
//! content layer is the surface in its own points, scaled into its parent's by a transform, so one
//! scale moves the group: the clip layers of its children first, then its buffer's layer on top,
//! whose contents are the `IOSurface` its producer presented last. A surface joins the layer tree
//! of the view whose scene first places it, as a sublayer of the view's layer below the
//! `CAMetalLayer` wgpu draws the framework into (raw-window-metal makes that one a sublayer too,
//! of an `NSView` and a `UIView` alike), so the clear rectangle the framework paints where the
//! surface goes shows it through; a child joins its parent's content layer where its parent's
//! present places it, below the parent's buffer, showing through its clear pixels. Every layer's
//! coordinates are the window's physical pixels over its scale, as a scene's placements are
//! pixels: a buffer shows whole at the scale its surface is placed at, and smaller under a
//! transform that shrinks it. The experiments in the `experiments` folder beside this repository
//! settled the choices here: `2026-09-22-layer-under-a-hole` the layer beneath the window's content
//! and its contents changed off the main thread, and `2026-09-25-buffer-hold` how long the display
//! engine holds a buffer.
//!
//! The module holds the compositor and the surface; `buffers` what a surface was presented and
//! when each goes back to its producer, `placed` the surfaces a scene or a present placed last,
//! and `window_surfaces` placing a scene's surfaces in a window.
//!
//! Two things touch a surface. Its producer presents from its own thread, in an explicit
//! transaction, placing its children in the same one. The main thread places it with the
//! window's frame. A producer that paces by the screen's refreshes waits on the platform's vsync
//! waiter, as the framework's frames do (`vsync_waiter.rs`); nothing of the surface's is driven by
//! the screen. Core Animation says only whether the compositor still reads a buffer, not when it
//! stops, so a surface asks at each present, of every buffer that was replaced, and a producer
//! never waits for one to come back: it draws into one it knows is free, or a new one, as
//! Flutter's `FlutterSurfaceManager` (`2026-09-25-surface-buffers-practice`) and Firefox's
//! `SurfacePoolCA` do. The invariant a later editor must keep: a buffer's release callback runs
//! only once `IOSurfaceIsInUse` answers false for it, or when its surface is gone.

// Core Animation's layers and the buffers they show need `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

mod buffers;
mod placed;
mod window_surfaces;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use inset_embedder::{Buffer, Compositor, Rect, Surface, SurfacePlacement, ViewId};
use objc2::rc::Retained;
use objc2_core_foundation::CGAffineTransform;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::{CALayer, CATransaction};

use buffers::{CrossThreadBuffer, PresentedBuffers, notify_released};
use placed::PlacedSurfaces;

pub(crate) use window_surfaces::{CoreAnimationWindowSurfaces, WindowLayer};

const HELD_BRIEFLY: &str = "a surface's state is never held across a panic";

/// The compositor of macOS and iOS: every surface is a layer of its own, so it keeps nothing.
pub(crate) struct CoreAnimationCompositor;

impl Compositor for CoreAnimationCompositor {
    fn open_surface(&self, view: ViewId, opaque: bool) -> Arc<dyn Surface> {
        Arc::new(CoreAnimationSurface::new(Placer::View(view), opaque))
    }
}

/// Who may place a surface: the scenes of one view, or the presents of one surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placer {
    View(ViewId),
    /// The surface of this number, its parent.
    Surface(u64),
}

/// A surface: its layers, the buffers presented to it and not yet released, and the children
/// its last present placed.
pub(crate) struct CoreAnimationSurface {
    placer: Placer,
    /// What a child names its parent by.
    number: u64,
    /// What a placement shows of the surface: the layer that joins the parent's tree, masking
    /// [`content_layer`](Self::content_layer) to its bounds.
    clip_layer: CrossThreadLayer,
    /// The surface in its own points, scaled into its parent's by a transform: its children's
    /// clip layers, then [`buffer_layer`](Self::buffer_layer) on top.
    content_layer: CrossThreadLayer,
    buffer_layer: CrossThreadLayer,
    buffers: Mutex<PresentedBuffers<CrossThreadBuffer>>,
    children: Mutex<Children>,
    /// Whether a placement by another placer was reported already.
    misplacement_reported: AtomicBool,
}

/// The children a surface's last present placed, and the scale of the window they were laid
/// out in: a window's pixels are its points times its scale, so a child is laid out again when
/// its parent comes to a window of another scale.
struct Children {
    placed: PlacedSurfaces,
    /// `None` until the surface is placed in a window; its children wait for it.
    window_scale: Option<f64>,
}

impl CoreAnimationSurface {
    fn new(placer: Placer, opaque: bool) -> CoreAnimationSurface {
        static NEXT_NUMBER: AtomicU64 = AtomicU64::new(0);
        let buffer_layer = CALayer::new();
        buffer_layer.setOpaque(opaque);
        // A view's layer is flipped (winit's `NSView`) or laid out from the top-left (a `UIView`'s):
        // a sublayer's frame is laid out from the top-left, but its contents gravity is not
        // flipped with it, so bottom-left is the visual top-left. A buffer smaller than the
        // layer, drawn before a resize reached the producer, shows at its own size from there
        // rather than stretched.
        // SAFETY: a constant Core Animation exports for the life of the process.
        buffer_layer.setContentsGravity(unsafe { objc2_quartz_core::kCAGravityBottomLeft });
        let content_layer = CALayer::new();
        // Laid out from its top-left corner, which the transform scales about.
        content_layer.setAnchorPoint(NSPoint::new(0.0, 0.0));
        content_layer.addSublayer(&buffer_layer);
        let clip_layer = CALayer::new();
        clip_layer.setMasksToBounds(true);
        clip_layer.addSublayer(&content_layer);
        clip_layer.setHidden(true);
        CoreAnimationSurface {
            placer,
            number: NEXT_NUMBER.fetch_add(1, Ordering::Relaxed),
            clip_layer: CrossThreadLayer(clip_layer),
            content_layer: CrossThreadLayer(content_layer),
            buffer_layer: CrossThreadLayer(buffer_layer),
            buffers: Mutex::new(PresentedBuffers::new()),
            children: Mutex::new(Children {
                placed: PlacedSurfaces::new(),
                window_scale: None,
            }),
            misplacement_reported: AtomicBool::new(false),
        }
    }

    /// The host's own surface behind a placed one; `None` for a surface no compositor of this
    /// host opened.
    fn downcast_from(surface: &Arc<dyn Surface>) -> Option<Arc<CoreAnimationSurface>> {
        let surface = Arc::clone(surface);
        let any: Arc<dyn std::any::Any + Send + Sync> = surface;
        any.downcast().ok()
    }

    /// Whether `placer` may place the surface, reporting once when it may not.
    fn may_be_placed_by(&self, placer: Placer) -> bool {
        if placer == self.placer {
            return true;
        }
        if !self.misplacement_reported.swap(true, Ordering::Relaxed) {
            eprintln!(
                "inset: a surface of {:?} was placed by {:?}; it is not shown",
                self.placer, placer
            );
        }
        false
    }

    /// Puts the clip layer at `index` among `parent`'s sublayers and shows the surface where
    /// `placement` says, in a window `window_scale` of whose physical pixels make a point: a
    /// view's layer, whose units are points, or a parent's content layer, whose units are the
    /// parent's own points. Inside the caller's transaction.
    fn place(&self, parent: &CALayer, index: u32, placement: &SurfacePlacement, window_scale: f64) {
        let Some(geometry) = LayerGeometry::of(placement, window_scale) else {
            self.hide();
            return;
        };
        let clip_layer = &self.clip_layer.0;
        if !is_sublayer_at(parent, clip_layer, index) {
            parent.insertSublayer_atIndex(clip_layer, index);
        }
        clip_layer.setFrame(geometry.clip_frame);
        let content_layer = &self.content_layer.0;
        content_layer.setPosition(geometry.content_origin);
        content_layer.setBounds(NSRect::new(NSPoint::new(0.0, 0.0), geometry.content_size));
        content_layer.setAffineTransform(scaling(geometry.content_scale));
        let buffer_layer = &self.buffer_layer.0;
        buffer_layer.setFrame(content_layer.bounds());
        buffer_layer.setContentsScale(window_scale);
        self.lay_out_children_for(window_scale);
        clip_layer.setHidden(false);
    }

    fn hide(&self) {
        self.clip_layer.0.setHidden(true);
    }

    /// Lays the children out again for a window of `window_scale`, if they were laid out for
    /// another.
    fn lay_out_children_for(&self, window_scale: f64) {
        let mut children = self.children.lock().expect(HELD_BRIEFLY);
        if children.window_scale == Some(window_scale) {
            return;
        }
        children.window_scale = Some(window_scale);
        children.placed.lay_out(&self.content_layer.0, window_scale);
    }

    /// Makes `placements` the surface's children and lays them out, where the surface has been
    /// placed in a window; hides the children no longer placed. Inside the caller's transaction.
    fn place_children(&self, placements: &[SurfacePlacement]) {
        let children = surfaces_placed_by(placements, Placer::Surface(self.number));
        let mut state = self.children.lock().expect(HELD_BRIEFLY);
        let window_scale = state.window_scale;
        state
            .placed
            .replace(children, &self.content_layer.0, window_scale);
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
    /// Shows `buffer` and places `children` in one transaction, and releases the replaced
    /// buffers the compositor has finished with.
    fn present(
        &self,
        buffer: Buffer,
        children: &[SurfacePlacement],
        released: Box<dyn FnOnce() + Send + Sync>,
    ) {
        let buffer = CrossThreadBuffer(buffer);
        in_transaction(|| {
            buffer.set_as_contents_of(&self.buffer_layer.0);
            self.place_children(children);
        });
        self.buffers
            .lock()
            .expect(HELD_BRIEFLY)
            .present(buffer, released);
        self.release_unused_buffers();
    }

    fn open_child(&self, opaque: bool) -> Arc<dyn Surface> {
        Arc::new(CoreAnimationSurface::new(
            Placer::Surface(self.number),
            opaque,
        ))
    }
}

impl Drop for CoreAnimationSurface {
    /// The layers leave their parent, the children go unless something else holds them, and
    /// every buffer goes back to the producer.
    fn drop(&mut self) {
        in_transaction(|| self.clip_layer.0.removeFromSuperlayer());
        let released = self.buffers.get_mut().expect(HELD_BRIEFLY).release_all();
        notify_released(released);
    }
}

/// This host's surfaces `placer` may place among `placements`, with their placements, in paint
/// order.
fn surfaces_placed_by(
    placements: &[SurfacePlacement],
    placer: Placer,
) -> Vec<(Arc<CoreAnimationSurface>, SurfacePlacement)> {
    placements
        .iter()
        .filter_map(|placement| {
            let surface =
                CoreAnimationSurface::downcast_from(&placement.surface).or_else(|| {
                    placed::report_foreign_surface();
                    None
                })?;
            surface
                .may_be_placed_by(placer)
                .then(|| (surface, placement.clone()))
        })
        .collect()
}

/// Where one placement puts a surface's layers, in its parent layer's points.
#[derive(Debug, PartialEq)]
struct LayerGeometry {
    /// The clip layer's frame: what shows.
    clip_frame: NSRect,
    /// The content layer's top-left corner, inside the clip layer.
    content_origin: NSPoint,
    /// The content layer's bounds: the surface in its own points.
    content_size: NSSize,
    /// How many of the parent's points one of the surface's own spans.
    content_scale: f64,
}

impl LayerGeometry {
    /// The layers of a surface placed by `placement` in a window `window_scale` pixels to the
    /// point: a producer draws at the window's scale, and the transforms above the surface shrink
    /// or grow what it drew by `placement.scale / window_scale`. `None` for a surface shrunk to
    /// nothing.
    fn of(placement: &SurfacePlacement, window_scale: f64) -> Option<LayerGeometry> {
        let content_scale = placement.scale / window_scale;
        if !(content_scale > 0.0 && content_scale.is_finite()) {
            return None;
        }
        let (rect, clip) = (placement.rect, placement.clip);
        Some(LayerGeometry {
            clip_frame: points_of(clip, window_scale),
            content_origin: NSPoint::new(
                (rect.left - clip.left) / window_scale,
                (rect.top - clip.top) / window_scale,
            ),
            content_size: NSSize::new(
                rect.width() / placement.scale,
                rect.height() / placement.scale,
            ),
            content_scale,
        })
    }
}

/// Whether `layer` is `parent`'s sublayer at `index` already.
fn is_sublayer_at(parent: &CALayer, layer: &CALayer, index: u32) -> bool {
    // SAFETY: reading a layer's sublayers on the thread that changes them: the main thread for a
    // view's layer, the parent's present, under its lock, for a content layer.
    let Some(sublayers) = (unsafe { parent.sublayers() }) else {
        return false;
    };
    (index as usize) < sublayers.count()
        && std::ptr::eq(&*sublayers.objectAtIndex(index as usize), layer)
}

/// `rect`, in a window's physical pixels, in points: `scale` pixels each.
fn points_of(rect: Rect, scale: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.left / scale, rect.top / scale),
        NSSize::new(rect.width() / scale, rect.height() / scale),
    )
}

/// A transform that scales by `scale` about the layer's anchor.
fn scaling(scale: f64) -> CGAffineTransform {
    CGAffineTransform {
        a: scale,
        b: 0.0,
        c: 0.0,
        d: scale,
        tx: 0.0,
        ty: 0.0,
    }
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
        let frame = points_of(Rect::from_ltwh(240.0, 180.0, 800.0, 600.0), 2.0);
        assert_eq!(frame.origin, NSPoint::new(120.0, 90.0));
        assert_eq!(frame.size, NSSize::new(400.0, 300.0));
    }

    #[test]
    fn a_surface_is_scaled_as_its_placement_is() {
        let geometry = |scale| {
            LayerGeometry::of(
                &unclipped(Rect::from_ltwh(0.0, 0.0, 800.0, 600.0), scale),
                2.0,
            )
        };
        let whole = geometry(2.0).unwrap();
        assert_eq!(
            (whole.content_size, whole.content_scale),
            (NSSize::new(400.0, 300.0), 1.0),
            "unscaled, a point of the surface is a point of the window"
        );
        let half = geometry(1.0).unwrap();
        assert_eq!(
            (half.content_size, half.content_scale),
            (NSSize::new(800.0, 600.0), 0.5),
            "at half size, the surface keeps its own points and the transform halves them"
        );
        assert_eq!(geometry(0.0), None);
    }

    /// A card: a page 390 by 844 points, at half its size, of which a square in the middle
    /// shows. In a flipped view, as winit's `NSView` is, the buffer's layer lies where the
    /// placement says whatever the layers between them, and its contents are flipped as a layer
    /// directly in the view would be, which is what makes bottom-left gravity the visual
    /// top-left.
    #[test]
    fn a_placed_surface_lies_where_its_placement_says_inside_its_clip() {
        let view_layer = flipped_view_layer();
        let surface = CoreAnimationSurface::new(Placer::View(ViewId(0)), true);
        let placement = SurfacePlacement {
            surface: idle_surface(),
            rect: Rect::from_ltwh(100.0, 200.0, 390.0, 844.0),
            scale: 1.0,
            clip: Rect::from_ltwh(100.0, 427.0, 390.0, 390.0),
        };
        surface.place(&view_layer, 0, &placement, 2.0);

        let buffer_layer = &surface.buffer_layer.0;
        assert_eq!(
            frame_in(buffer_layer, &view_layer),
            NSRect::new(NSPoint::new(50.0, 100.0), NSSize::new(195.0, 422.0))
        );
        let clip_layer = &surface.clip_layer.0;
        assert_eq!(clip_layer.frame().origin, NSPoint::new(50.0, 213.5));
        assert!(clip_layer.masksToBounds() && !clip_layer.isHidden());
        assert_eq!(buffer_layer.contentsScale(), 2.0);

        let directly_in_view = CALayer::new();
        view_layer.addSublayer(&directly_in_view);
        assert_eq!(
            buffer_layer.contentsAreFlipped(),
            directly_in_view.contentsAreFlipped()
        );
    }

    /// A program's picture placed at half size in a window, with its video beneath it: the
    /// child's placement is in the parent's pixels, and the parent's scale and clip carry it.
    #[test]
    fn a_child_lies_beneath_its_parent_where_the_parent_places_it_scaled_with_it() {
        let view_layer = flipped_view_layer();
        let parent = Arc::new(CoreAnimationSurface::new(Placer::View(ViewId(0)), false));
        let child = parent.open_child(true);
        let child_layers = CoreAnimationSurface::downcast_from(&child).unwrap();
        in_transaction(|| {
            parent.place_children(&[unclipped_surface(
                Arc::clone(&child),
                Rect::from_ltwh(200.0, 100.0, 400.0, 300.0),
                2.0,
            )])
        });
        assert!(
            child_layers.clip_layer.0.superlayer().is_none(),
            "a child waits for its parent to be in a window"
        );

        parent.place(
            &view_layer,
            0,
            &SurfacePlacement {
                surface: idle_surface(),
                rect: Rect::from_ltwh(100.0, 100.0, 1000.0, 800.0),
                scale: 1.0,
                clip: Rect::from_ltwh(100.0, 100.0, 1000.0, 800.0),
            },
            2.0,
        );
        assert_eq!(
            frame_in(&child_layers.buffer_layer.0, &view_layer),
            NSRect::new(NSPoint::new(100.0, 75.0), NSSize::new(100.0, 75.0)),
            "the parent's half scale halves the child's place and size"
        );
        let content_sublayers = unsafe { parent.content_layer.0.sublayers() }.unwrap();
        assert!(std::ptr::eq(
            &*content_sublayers.objectAtIndex(0),
            &*child_layers.clip_layer.0
        ));
        assert!(
            std::ptr::eq(
                &*content_sublayers.lastObject().unwrap(),
                &*parent.buffer_layer.0
            ),
            "the parent's buffer is above its children"
        );
    }

    #[test]
    fn a_child_its_parent_no_longer_places_is_hidden_and_a_strangers_never_shows() {
        let parent = CoreAnimationSurface::new(Placer::View(ViewId(0)), false);
        let child = parent.open_child(true);
        let child_layers = CoreAnimationSurface::downcast_from(&child).unwrap();
        let rect = Rect::from_ltwh(0.0, 0.0, 10.0, 10.0);
        in_transaction(|| {
            parent.lay_out_children_for(2.0);
            parent.place_children(&[unclipped_surface(Arc::clone(&child), rect, 2.0)]);
        });
        assert!(!child_layers.clip_layer.0.isHidden());
        in_transaction(|| parent.place_children(&[]));
        assert!(child_layers.clip_layer.0.isHidden());

        let stranger = CoreAnimationSurface::new(Placer::View(ViewId(0)), true);
        let stranger: Arc<dyn Surface> = Arc::new(stranger);
        in_transaction(|| parent.place_children(&[unclipped_surface(stranger.clone(), rect, 2.0)]));
        let stranger_layers = CoreAnimationSurface::downcast_from(&stranger).unwrap();
        assert!(
            stranger_layers.clip_layer.0.superlayer().is_none(),
            "a surface opened for a view is placed by its scenes alone"
        );
    }

    /// A placement's own surface, which [`CoreAnimationSurface::place`] does not read.
    fn idle_surface() -> Arc<dyn Surface> {
        Arc::new(CoreAnimationSurface::new(Placer::View(ViewId(0)), true))
    }

    /// The layer of a view winit makes on macOS: flipped, so laid out from the top-left.
    fn flipped_view_layer() -> Retained<CALayer> {
        let view_layer = CALayer::new();
        view_layer.setGeometryFlipped(true);
        view_layer
    }

    /// Where `layer`'s bounds lie in `ancestor`'s coordinates.
    fn frame_in(layer: &CALayer, ancestor: &CALayer) -> NSRect {
        layer.convertRect_toLayer(layer.bounds(), Some(ancestor))
    }

    /// An idle surface at `rect`, all of it showing, at `scale`.
    fn unclipped(rect: Rect, scale: f64) -> SurfacePlacement {
        unclipped_surface(idle_surface(), rect, scale)
    }

    fn unclipped_surface(surface: Arc<dyn Surface>, rect: Rect, scale: f64) -> SurfacePlacement {
        SurfacePlacement {
            surface,
            rect,
            scale,
            clip: rect,
        }
    }
}

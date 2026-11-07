//! Composition layers as Core Animation layers beneath a window's content, on macOS and iOS alike.
//!
//! A composition layer is three `CALayer`s. Its clip layer cuts it to the bounds of what shows, and
//! is masked by the shape of any clip above it that is not an upright rectangle. Inside, its
//! content layer is the composition layer in its own coordinates, taken into its parent's by every
//! transform above it, so one transform moves the group: the clip layers of its children first,
//! then its content on top, either its buffer's layer, whose contents are the `IOSurface` its
//! producer presented last, or a layer the system draws into, a player's or a camera's preview. A
//! layer a view's scene places lies below the `CAMetalLayer` the view draws into, so the room the
//! scene's picture makes for it shows it through: on macOS in the content of the view's own
//! composition layer, whose content that `CAMetalLayer` is (`view_layer`), and on iOS in the layer
//! of winit's `UIView`, where wgpu makes the `CAMetalLayer` from the window's handle
//! (`window_composition`). A child joins its parent's content layer where its parent's present
//! places it, below the parent's buffer, showing through its clear pixels; and on macOS a layer the
//! app places goes into a `CALayer` the app holds, such as the root of a `CAContext` another process
//! shows, as a window places its view's layer in the layer of winit's `NSView`. Every layer's
//! coordinates are the window's physical pixels over its scale, as a scene's
//! placements are pixels: a buffer shows whole at the scale its layer is placed at, and smaller
//! under a transform that shrinks it. The experiments in the `experiments` folder beside this
//! repository settled the choices here: `2026-09-22-layer-under-a-hole` the layer beneath the
//! window's content and its contents changed off the main thread, and `2026-09-25-buffer-hold`
//! how long the display engine holds a buffer.
//!
//! The module holds the compositor and the composition layer; `layer_geometry` where a placement
//! puts a layer's `CALayer`s, `buffers` what a layer was presented and when each goes back to its
//! producer, `placed` the layers a scene or a present placed last, `view_layer` the composition
//! layer a view draws into on macOS, and `window_composition` placing a scene's layers in a window
//! on iOS.
//!
//! Two things touch a layer. Its producer presents it from its own thread, with its children, in
//! an explicit transaction. The main thread places it with the
//! window's frame. A producer that paces by the screen's refreshes waits on the platform's vsync
//! waiter, as the framework's frames do (`vsync_waiter.rs`); nothing of the layer's is driven by
//! the screen. Core Animation says only whether the compositor still reads a buffer, not when it
//! stops, so a layer asks at each present, of every buffer that was replaced, and a producer
//! never waits for one to come back: it draws into one it knows is free, or a new one, as
//! Flutter's `FlutterSurfaceManager` (`2026-09-25-surface-buffers-practice`) and Firefox's
//! `SurfacePoolCA` do. The invariants a later editor must keep: a buffer's release callback runs
//! only once `IOSurfaceIsInUse` answers false for it, or when its layer is gone; and a layer has
//! one placer at a time, from the placement that adopts it until that placer lets it go.

// Core Animation's layers and the buffers they show need `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

mod buffers;
mod layer_geometry;
mod placed;
#[cfg(target_os = "macos")]
mod view_layer;
#[cfg(target_os = "ios")]
mod window_composition;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use objc2::Message;
use objc2::rc::Retained;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::CGMutablePath;
use objc2_foundation::{NSPoint, NSRect};
use objc2_quartz_core::{CALayer, CAShapeLayer, CATransaction};
use waft_embedder::{CompositionLayer, Compositor, HardwareBuffer, Placement, ViewId};
use winit::window::Window;

use buffers::{CrossThreadBuffer, PresentedBuffers, notify_released};
use layer_geometry::LayerGeometry;
use placed::PlacedLayers;

#[cfg(target_os = "macos")]
pub(crate) use view_layer::CoreAnimationViewLayer;
#[cfg(target_os = "ios")]
pub(crate) use window_composition::CoreAnimationWindowComposition;

const HELD_BRIEFLY: &str = "a layer's state is never held across a panic";

/// The compositor of macOS and iOS: every composition layer is a layer of its own, so it keeps
/// nothing.
pub(crate) struct CoreAnimationCompositor;

impl Compositor for CoreAnimationCompositor {
    fn make_layer(&self, opaque: bool) -> Arc<dyn CompositionLayer> {
        Arc::new(CoreAnimationLayer::new(opaque))
    }
}

/// Where a system's window keeps the layers of its content: the layer of winit's view.
pub(crate) trait WindowLayer {
    /// The layer of `window`'s view; `None` off the main thread, or for a view without one.
    fn window_layer(window: &dyn Window) -> Option<Retained<CALayer>>;
}

/// Who places a layer: the scenes of one view, the presents of one layer, the app, in a `CALayer`
/// of its own, or a window, the layer its view draws into in the layer of winit's view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placer {
    View(ViewId),
    /// The layer of this number, its parent.
    Layer(u64),
    #[cfg(target_os = "macos")]
    App,
    #[cfg(target_os = "macos")]
    Window,
}

/// A composition layer: its `CALayer`s, who places it, the buffers presented to it and not yet
/// released, and its children.
pub(crate) struct CoreAnimationLayer {
    /// What a child names its parent by.
    number: u64,
    /// The placer holding the layer, from the placement that adopted it until it lets the layer
    /// go; `None` while nothing places it.
    placer: Mutex<Option<Placer>>,
    /// What a placement shows of the layer: the `CALayer` that joins the parent's tree, masking
    /// [`content_layer`](Self::content_layer) to its bounds and to the shapes of the clips above.
    clip_layer: CrossThreadLayer,
    /// The layer in its own coordinates, taken into the clip layer's by every transform above
    /// it: its children's clip layers, then its content on top.
    content_layer: CrossThreadLayer,
    buffer_layer: CrossThreadLayer,
    /// A layer the system draws into, the content in place of the buffers while it is shown.
    system_layer: Mutex<Option<CrossThreadLayer>>,
    buffers: Mutex<PresentedBuffers<CrossThreadBuffer>>,
    children: Mutex<Children>,
    /// Whether a placement by a second placer was reported already.
    misplacement_reported: AtomicBool,
    /// Whether it was made opaque: its buffer is drawn so while no children lie beneath it.
    opaque: bool,
}

/// The children a layer's last present showed, and the scale of the window they were laid out
/// in: a window's pixels are its points times its scale, so a child is laid out again when its
/// parent comes to a window of another scale.
struct Children {
    placed: PlacedLayers,
    /// `None` until the layer is placed in a window; its children wait for it.
    window_scale: Option<f64>,
}

impl CoreAnimationLayer {
    /// A layer placed nowhere, made in a transaction of its own: a compositor is asked for layers
    /// on any thread.
    fn new(opaque: bool) -> CoreAnimationLayer {
        in_transaction(|| {
            static NEXT_NUMBER: AtomicU64 = AtomicU64::new(0);
            let buffer_layer = CALayer::new();
            buffer_layer.setOpaque(opaque);
            // A view's layer is flipped (winit's `NSView`) or laid out from the top-left (a
            // `UIView`'s): a sublayer's frame is laid out from the top-left, but its contents
            // gravity is not flipped with it, so bottom-left is the visual top-left. A buffer
            // smaller than the layer, drawn before a resize reached the producer, shows at its own
            // size from there rather than stretched. SAFETY: a constant Core Animation exports for
            // the life of the process.
            buffer_layer.setContentsGravity(unsafe { objc2_quartz_core::kCAGravityBottomLeft });
            let content_layer = CALayer::new();
            // Laid out from its top-left corner at the clip layer's: the transform places it.
            content_layer.setAnchorPoint(NSPoint::new(0.0, 0.0));
            content_layer.setPosition(NSPoint::new(0.0, 0.0));
            content_layer.addSublayer(&buffer_layer);
            let clip_layer = CALayer::new();
            clip_layer.setMasksToBounds(true);
            clip_layer.addSublayer(&content_layer);
            clip_layer.setHidden(true);
            CoreAnimationLayer {
                number: NEXT_NUMBER.fetch_add(1, Ordering::Relaxed),
                placer: Mutex::new(None),
                clip_layer: CrossThreadLayer(clip_layer),
                content_layer: CrossThreadLayer(content_layer),
                buffer_layer: CrossThreadLayer(buffer_layer),
                system_layer: Mutex::new(None),
                buffers: Mutex::new(PresentedBuffers::new()),
                children: Mutex::new(Children {
                    placed: PlacedLayers::new(),
                    window_scale: None,
                }),
                misplacement_reported: AtomicBool::new(false),
                opaque,
            }
        })
    }

    /// The host's own layer behind a placed one; `None` for a layer no compositor of this host
    /// made.
    fn downcast_from(layer: &Arc<dyn CompositionLayer>) -> Option<Arc<CoreAnimationLayer>> {
        let layer = Arc::clone(layer);
        let any: Arc<dyn std::any::Any + Send + Sync> = layer;
        any.downcast().ok()
    }

    /// Whether `placer` holds the layer, adopting it when nothing does, and reporting once when
    /// another placer does.
    fn adopted_by(&self, placer: Placer) -> bool {
        let mut holder = self.placer.lock().expect(HELD_BRIEFLY);
        if *holder.get_or_insert(placer) == placer {
            return true;
        }
        if !self.misplacement_reported.swap(true, Ordering::Relaxed) {
            eprintln!(
                "waft: a composition layer {:?} holds was placed by {:?}; it is not shown there",
                *holder, placer
            );
        }
        false
    }

    /// Its placer lets it go: it is hidden until a placer adopts it again. Inside the caller's
    /// transaction.
    fn let_go(&self) {
        *self.placer.lock().expect(HELD_BRIEFLY) = None;
        self.clip_layer.0.setHidden(true);
    }

    /// Puts the clip layer at `index` among `parent`'s sublayers and shows the layer where
    /// `placement` says, in a window `window_scale` of whose physical pixels make a point: a
    /// view's layer, whose units are points, or a parent's content layer, whose units are the
    /// parent's own points. Inside the caller's transaction.
    fn show_in(&self, parent: &CALayer, index: u32, placement: &Placement, window_scale: f64) {
        let Some(geometry) = LayerGeometry::of(placement, window_scale) else {
            self.clip_layer.0.setHidden(true);
            return;
        };
        let clip_layer = &self.clip_layer.0;
        if !is_sublayer_at(parent, clip_layer, index) {
            parent.insertSublayer_atIndex(clip_layer, index);
        }
        clip_layer.setFrame(geometry.clip_frame);
        mask_to_shapes(clip_layer, &geometry.clip_shapes);
        let content_layer = &self.content_layer.0;
        content_layer.setBounds(NSRect::new(NSPoint::new(0.0, 0.0), geometry.content_size));
        content_layer.setTransform(geometry.content_transform);
        let buffer_layer = &self.buffer_layer.0;
        buffer_layer.setFrame(content_layer.bounds());
        buffer_layer.setContentsScale(window_scale);
        if let Some(system_layer) = &*self.system_layer.lock().expect(HELD_BRIEFLY)
            && is_sublayer_of(&system_layer.0, content_layer)
        {
            system_layer.0.setFrame(content_layer.bounds());
        }
        self.lay_out_children_for(window_scale);
        clip_layer.setHidden(false);
    }

    /// Shows `system_layer`, which the system draws into, as the layer's content, filling it, in
    /// place of the buffer presented last and of any system layer shown before. Main thread.
    fn show_system_layer(&self, system_layer: &CALayer) {
        let mut shown = self.system_layer.lock().expect(HELD_BRIEFLY);
        in_transaction(|| {
            let content_layer = &self.content_layer.0;
            if let Some(previous) = shown.take()
                && !std::ptr::eq(&*previous.0, system_layer)
                && is_sublayer_of(&previous.0, content_layer)
            {
                previous.0.removeFromSuperlayer();
            }
            // SAFETY: clearing a layer's contents, inside the transaction, on the main thread.
            unsafe { self.buffer_layer.0.setContents(None) };
            system_layer.setFrame(content_layer.bounds());
            content_layer.addSublayer(system_layer);
        });
        *shown = Some(CrossThreadLayer(system_layer.retain()));
        self.buffers.lock().expect(HELD_BRIEFLY).take_off_screen();
        self.release_unused_buffers();
    }

    /// Takes down the system layer shown as the content, for a buffer to take its place. Inside
    /// the caller's transaction.
    fn take_down_system_layer(&self) {
        if let Some(system_layer) = self.system_layer.lock().expect(HELD_BRIEFLY).take()
            && is_sublayer_of(&system_layer.0, &self.content_layer.0)
        {
            system_layer.0.removeFromSuperlayer();
        }
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

    /// Makes `children` the ones shown, laid out where the layer has been placed in a window,
    /// and lets go of the ones no longer shown; the buffer above them is blended while there are
    /// any, since beneath an opaque one they would never show. Inside the caller's transaction.
    fn place_children(&self, children: Vec<(Arc<CoreAnimationLayer>, Placement)>) {
        self.buffer_layer
            .0
            .setOpaque(self.opaque && children.is_empty());
        let adopted = adopted_by(children, Placer::Layer(self.number));
        let mut shown = self.children.lock().expect(HELD_BRIEFLY);
        let window_scale = shown.window_scale;
        shown
            .placed
            .replace(adopted, &self.content_layer.0, window_scale);
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

impl CompositionLayer for CoreAnimationLayer {
    /// Shows the buffer and places the children in one transaction, and releases the replaced
    /// buffers the compositor has finished with.
    fn present(
        &self,
        buffer: HardwareBuffer,
        children: Vec<(Arc<dyn CompositionLayer>, Placement)>,
        released: Box<dyn FnOnce() + Send + Sync>,
    ) {
        let buffer = CrossThreadBuffer(buffer);
        let children = own_layers(children.iter().map(|(layer, placement)| (layer, placement)));
        in_transaction(|| {
            self.take_down_system_layer();
            buffer.set_as_contents_of(&self.buffer_layer.0);
            self.place_children(children);
        });
        self.buffers
            .lock()
            .expect(HELD_BRIEFLY)
            .present(buffer, released);
        self.release_unused_buffers();
    }
}

impl Drop for CoreAnimationLayer {
    /// The `CALayer`s leave their parent, the children are let go, and every buffer goes back
    /// to the producer.
    fn drop(&mut self) {
        in_transaction(|| self.clip_layer.0.removeFromSuperlayer());
        let released = self.buffers.get_mut().expect(HELD_BRIEFLY).release_all();
        notify_released(released);
    }
}

/// Shows `system_layer`, which the system draws into, such as a player's, as `layer`'s content,
/// filling it and clipped and scaled with it, in place of its buffers and of any system layer
/// shown before. A layer no compositor of this host made shows nothing. Main thread.
pub(crate) fn show_system_layer(layer: &Arc<dyn CompositionLayer>, system_layer: &CALayer) {
    match CoreAnimationLayer::downcast_from(layer) {
        Some(layer) => layer.show_system_layer(system_layer),
        None => placed::report_foreign_layer(),
    }
}

/// Places `layer` in `parent`, a `CALayer` the app holds, above its sublayers, where `placement`
/// says, as a view's scene places a layer: its rectangle in the layer's own coordinates under
/// transforms that take them to pixels, `scale` of which make one of `parent`'s points. The app holds the layer from then on, until the layer goes. A layer no
/// compositor of this host made, or one another placer holds, shows nothing. Main thread.
#[cfg(target_os = "macos")]
pub(crate) fn place_layer_in(
    parent: &CALayer,
    layer: &Arc<dyn CompositionLayer>,
    placement: &Placement,
    scale: f64,
) {
    let Some(layer) = CoreAnimationLayer::downcast_from(layer) else {
        placed::report_foreign_layer();
        return;
    };
    place_on_top(parent, &layer, placement, scale, Placer::App);
}

/// Places `layer` in `parent`, the layer of a window's view, filling the bounds AppKit gives it, at
/// `scale` pixels to its point: how a window places the layer its view draws into. The rectangle is
/// in the view's logical pixels under a transform by `scale`, as a scene's root places what it
/// draws, so the layer's content is in the view's logical pixels, as `parent` is. A layer no
/// compositor of this host made, or one another placer holds, shows nothing. Main thread.
#[cfg(target_os = "macos")]
pub(crate) fn place_filling(parent: &CALayer, layer: &Arc<dyn CompositionLayer>, scale: f64) {
    let Some(layer) = CoreAnimationLayer::downcast_from(layer) else {
        placed::report_foreign_layer();
        return;
    };
    let bounds = parent.bounds();
    let filling = Placement {
        rect: waft_embedder::Rect::from_ltwh(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        ),
        mutators: vec![waft_embedder::Mutator::Transform(
            waft_embedder::Matrix4::scale(scale as f32, scale as f32),
        )],
    };
    place_on_top(parent, &layer, &filling, scale, Placer::Window);
}

/// Places `layer` in `parent`, a `CALayer` outside every composition layer, above its sublayers,
/// where `placement` says, in pixels `scale` of which make one of `parent`'s points, if `placer`
/// holds it or nothing does. Main thread.
#[cfg(target_os = "macos")]
fn place_on_top(
    parent: &CALayer,
    layer: &CoreAnimationLayer,
    placement: &Placement,
    scale: f64,
    placer: Placer,
) {
    if !layer.adopted_by(placer) {
        return;
    }
    let index = index_on_top(parent, &layer.clip_layer.0);
    in_transaction(|| layer.show_in(parent, index, placement, scale));
}

/// The layers among `placements` `placer` holds, adopting those nothing holds, in paint order.
fn adopted_by(
    placements: Vec<(Arc<CoreAnimationLayer>, Placement)>,
    placer: Placer,
) -> Vec<(Arc<CoreAnimationLayer>, Placement)> {
    placements
        .into_iter()
        .filter(|(layer, _)| layer.adopted_by(placer))
        .collect()
}

/// This host's layers among `placements`, a scene's, with their placements, in paint order.
fn own_layers<'a>(
    placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
) -> Vec<(Arc<CoreAnimationLayer>, Placement)> {
    placements
        .into_iter()
        .filter_map(|(layer, placement)| {
            let layer = CoreAnimationLayer::downcast_from(layer).or_else(|| {
                placed::report_foreign_layer();
                None
            })?;
            Some((layer, placement.clone()))
        })
        .collect()
}

/// Whether `layer` is `parent`'s sublayer at `index` already.
fn is_sublayer_at(parent: &CALayer, layer: &CALayer, index: u32) -> bool {
    // SAFETY: reading a layer's sublayers on the thread that changes them: the main thread for a
    // view's layer and an app's, the parent's present, under its lock, for a content layer.
    let Some(sublayers) = (unsafe { parent.sublayers() }) else {
        return false;
    };
    (index as usize) < sublayers.count()
        && std::ptr::eq(&*sublayers.objectAtIndex(index as usize), layer)
}

/// Where `layer` lies among `parent`'s sublayers, or past the last when it is not one: what
/// keeps a layer the app places above the sublayers it had.
#[cfg(target_os = "macos")]
fn index_on_top(parent: &CALayer, layer: &CALayer) -> u32 {
    // SAFETY: as in `is_sublayer_at`, on the main thread.
    let Some(sublayers) = (unsafe { parent.sublayers() }) else {
        return 0;
    };
    let count = sublayers.count();
    let index = (0..count)
        .find(|&index| std::ptr::eq(&*sublayers.objectAtIndex(index), layer))
        .unwrap_or(count);
    index as u32
}

/// Whether `layer` is shown in `parent` still: a system layer moves from one composition layer
/// to another.
fn is_sublayer_of(layer: &CALayer, parent: &CALayer) -> bool {
    layer
        .superlayer()
        .is_some_and(|superlayer| std::ptr::eq(&*superlayer, parent))
}

/// Masks `layer` to the intersection of `shapes`, in its own points, or to nothing beyond its
/// bounds when there are none: each shape a shape layer, the mask of the one before, so Core
/// Animation keeps only what lies inside every one.
fn mask_to_shapes(layer: &CALayer, shapes: &[CFRetained<CGMutablePath>]) {
    let bounds = NSRect::new(NSPoint::new(0.0, 0.0), layer.bounds().size);
    let mut mask: Option<Retained<CALayer>> = None;
    for shape in shapes.iter().rev() {
        let shape_layer = CAShapeLayer::new();
        shape_layer.setFrame(bounds);
        shape_layer.setPath(Some(shape));
        // SAFETY: a mask made here, in no layer tree, inside the caller's transaction.
        unsafe { shape_layer.setMask(mask.as_deref()) };
        mask = Some(Retained::into_super(shape_layer));
    }
    // SAFETY: the mask is in no layer tree, and the layer is changed inside the caller's
    // transaction, on the thread that places it.
    unsafe { layer.setMask(mask.as_deref()) };
}

/// Runs `change` in a transaction of its own with implicit animations off: a layer changed
/// off the main thread must be in one, and a layer the framework moves must not animate.
pub(crate) fn in_transaction<T>(change: impl FnOnce() -> T) -> T {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    let changed = change();
    CATransaction::commit();
    changed
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
    use objc2::ClassType;
    use objc2_foundation::{NSObjectProtocol, NSPoint, NSSize};
    use waft_embedder::{Matrix4, Mutator, RRect, Radius, Rect};

    use super::*;

    /// A card: a page 390 by 844 points, at half its size, of which a square in the middle
    /// shows. In a flipped view, as winit's `NSView` is, the buffer's layer lies where the
    /// placement says whatever the layers between them, and its contents are flipped as a layer
    /// directly in the view would be, which is what makes bottom-left gravity the visual
    /// top-left.
    #[test]
    fn a_placed_layer_lies_where_its_placement_says_inside_its_clip() {
        let view_layer = flipped_view_layer();
        let layer = CoreAnimationLayer::new(true);
        let placement = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 390.0, 844.0),
            mutators: vec![
                Mutator::ClipRect(Rect::from_ltwh(100.0, 427.0, 390.0, 390.0)),
                Mutator::Transform(Matrix4::translation(100.0, 200.0)),
            ],
        };
        layer.show_in(&view_layer, 0, &placement, 2.0);

        let buffer_layer = &layer.buffer_layer.0;
        assert_eq!(
            frame_in(buffer_layer, &view_layer),
            NSRect::new(NSPoint::new(50.0, 100.0), NSSize::new(195.0, 422.0))
        );
        let clip_layer = &layer.clip_layer.0;
        assert_eq!(clip_layer.frame().origin, NSPoint::new(50.0, 213.5));
        assert!(clip_layer.masksToBounds() && !clip_layer.isHidden());
        assert!(clip_layer.mask().is_none(), "a rectangle needs no mask");
        assert_eq!(buffer_layer.contentsScale(), 2.0);

        let directly_in_view = CALayer::new();
        view_layer.addSublayer(&directly_in_view);
        assert_eq!(
            buffer_layer.contentsAreFlipped(),
            directly_in_view.contentsAreFlipped()
        );
    }

    /// A layer turned a quarter: its `CALayer`s turn with it, about the corner the transform
    /// turns it about, where a rectangle can only bound it.
    #[test]
    fn a_turned_layer_turns_its_core_animation_layers() {
        let view_layer = flipped_view_layer();
        let layer = CoreAnimationLayer::new(true);
        let placement = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 200.0, 100.0),
            mutators: vec![
                Mutator::Transform(Matrix4::translation(300.0, 0.0)),
                Mutator::Transform(Matrix4::rotation(std::f32::consts::FRAC_PI_2)),
            ],
        };
        layer.show_in(&view_layer, 0, &placement, 2.0);
        let frame = frame_in(&layer.buffer_layer.0, &view_layer);
        let expected = NSRect::new(NSPoint::new(100.0, 0.0), NSSize::new(50.0, 100.0));
        assert!(
            (frame.origin.x - expected.origin.x).abs() < 1e-3
                && (frame.origin.y - expected.origin.y).abs() < 1e-3
                && (frame.size.width - expected.size.width).abs() < 1e-3
                && (frame.size.height - expected.size.height).abs() < 1e-3,
            "{frame:?}"
        );
    }

    /// Two rounded clips that both cut the layer mask it in turn: the clip layer's mask, and
    /// that mask's own; once neither cuts it, the mask goes.
    #[test]
    fn rounded_clips_that_cut_a_layer_mask_its_clip_layer_in_turn() {
        let view_layer = flipped_view_layer();
        let layer = CoreAnimationLayer::new(true);
        let rounded = |rect, radius| RRect::from_rect_and_radius(rect, Radius::circular(radius));
        let cut = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 100.0, 100.0),
            mutators: vec![
                Mutator::ClipRRect(rounded(Rect::from_ltwh(0.0, 0.0, 100.0, 100.0), 40.0)),
                Mutator::ClipRRect(rounded(Rect::from_ltwh(10.0, 10.0, 80.0, 80.0), 16.0)),
            ],
        };
        layer.show_in(&view_layer, 0, &cut, 2.0);
        let clip_layer = &layer.clip_layer.0;
        let outer = clip_layer.mask().expect("the outer clip's shape");
        let inner = outer
            .mask()
            .expect("the inner clip's shape, masking the outer");
        assert!(inner.mask().is_none());
        assert!(outer.isKindOfClass(CAShapeLayer::class()));
        assert_eq!(outer.frame(), clip_layer.bounds());

        let uncut = Placement {
            rect: Rect::from_ltwh(30.0, 30.0, 40.0, 40.0),
            ..cut
        };
        layer.show_in(&view_layer, 0, &uncut, 2.0);
        assert!(
            clip_layer.mask().is_none(),
            "no corner reaches the layer in the middle"
        );
    }

    #[test]
    fn a_layer_nothing_lets_show_is_hidden() {
        let view_layer = flipped_view_layer();
        let layer = CoreAnimationLayer::new(true);
        let placement = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            mutators: vec![Mutator::Opacity(0)],
        };
        layer.show_in(&view_layer, 0, &placement, 2.0);
        assert!(layer.clip_layer.0.isHidden());
        let half = Placement {
            mutators: vec![Mutator::Opacity(128)],
            ..placement
        };
        layer.show_in(&view_layer, 0, &half, 2.0);
        let clip_layer = &layer.clip_layer.0;
        assert!(!clip_layer.isHidden());
        assert_eq!(
            clip_layer.opacity(),
            1.0,
            "the opacity is the room's, not the layer's"
        );
    }

    /// A program's picture placed at half size in a window, with its video beneath it: the
    /// child's placement is in the parent's pixels, and the parent's transform and clip carry it.
    #[test]
    fn a_child_lies_beneath_its_parent_where_the_parent_places_it_scaled_with_it() {
        let view_layer = flipped_view_layer();
        let parent = CoreAnimationLayer::new(false);
        let video = Arc::new(CoreAnimationLayer::new(true));
        in_transaction(|| {
            parent.place_children(vec![(
                Arc::clone(&video),
                scaled(Rect::from_ltwh(200.0, 100.0, 400.0, 300.0), 2.0),
            )])
        });
        assert!(
            video.clip_layer.0.superlayer().is_none(),
            "a child waits for its parent to be in a window"
        );

        parent.show_in(
            &view_layer,
            0,
            &scaled(Rect::from_ltwh(100.0, 100.0, 1000.0, 800.0), 1.0),
            2.0,
        );
        assert_eq!(
            frame_in(&video.buffer_layer.0, &view_layer),
            NSRect::new(NSPoint::new(100.0, 75.0), NSSize::new(100.0, 75.0)),
            "the parent's half scale halves the child's place and size"
        );
        let content_sublayers = unsafe { parent.content_layer.0.sublayers() }.unwrap();
        assert!(std::ptr::eq(
            &*content_sublayers.objectAtIndex(0),
            &*video.clip_layer.0
        ));
        assert!(
            std::ptr::eq(
                &*content_sublayers.lastObject().unwrap(),
                &*parent.buffer_layer.0
            ),
            "the parent's buffer is above its children"
        );
    }

    /// A present shows exactly the children it names: one it no longer names is let go, free for
    /// another placer, and one another placer holds never shows.
    #[test]
    fn an_opaque_layer_blends_its_buffer_while_it_has_children() {
        let parent = CoreAnimationLayer::new(true);
        let video = Arc::new(CoreAnimationLayer::new(true));
        let child = vec![(video, scaled(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), 2.0))];
        in_transaction(|| parent.place_children(child));
        assert!(!parent.buffer_layer.0.isOpaque());

        in_transaction(|| parent.place_children(Vec::new()));
        assert!(parent.buffer_layer.0.isOpaque());
    }

    #[test]
    fn a_present_lets_go_of_the_children_it_no_longer_names_and_shows_no_one_elses() {
        let parent = CoreAnimationLayer::new(false);
        let video = Arc::new(CoreAnimationLayer::new(true));
        let child = || {
            vec![(
                Arc::clone(&video),
                scaled(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), 2.0),
            )]
        };
        in_transaction(|| {
            parent.lay_out_children_for(2.0);
            parent.place_children(child());
        });
        assert!(!video.clip_layer.0.isHidden());
        assert!(!video.adopted_by(Placer::View(ViewId(0))));

        in_transaction(|| parent.place_children(Vec::new()));
        assert!(video.clip_layer.0.isHidden());
        assert!(
            video.adopted_by(Placer::View(ViewId(0))),
            "let go, it is free for a view to place"
        );

        in_transaction(|| parent.place_children(child()));
        assert!(
            video.clip_layer.0.isHidden(),
            "a layer a view holds is not shown in another parent"
        );
    }

    #[test]
    fn a_layer_is_held_by_the_first_to_place_it_until_it_lets_it_go() {
        let layer = CoreAnimationLayer::new(true);
        assert!(layer.adopted_by(Placer::View(ViewId(3))));
        assert!(layer.adopted_by(Placer::View(ViewId(3))));
        assert!(!layer.adopted_by(Placer::View(ViewId(4))));
        in_transaction(|| layer.let_go());
        assert!(layer.adopted_by(Placer::View(ViewId(4))));
    }

    /// A parent that goes lets go of its children, for their producers to place elsewhere.
    #[test]
    fn a_parent_that_goes_lets_go_of_its_children() {
        let parent = CoreAnimationLayer::new(false);
        let video = Arc::new(CoreAnimationLayer::new(true));
        in_transaction(|| {
            parent.place_children(vec![(
                Arc::clone(&video),
                scaled(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), 2.0),
            )])
        });
        drop(parent);
        assert!(video.adopted_by(Placer::View(ViewId(0))));
    }

    /// A player's layer is the content in place of the buffer, filling the layer through every
    /// placement, and a second replaces the first.
    #[test]
    fn a_system_layer_is_the_content_in_place_of_the_buffer() {
        let view_layer = flipped_view_layer();
        let layer = made_layer();
        let layers = CoreAnimationLayer::downcast_from(&layer).unwrap();
        let first = CALayer::new();
        show_system_layer(&layer, &first);
        layers.show_in(
            &view_layer,
            0,
            &scaled(Rect::from_ltwh(20.0, 40.0, 640.0, 360.0), 2.0),
            2.0,
        );
        assert_eq!(
            frame_in(&first, &view_layer),
            NSRect::new(NSPoint::new(10.0, 20.0), NSSize::new(320.0, 180.0))
        );
        assert!(unsafe { layers.buffer_layer.0.contents() }.is_none());

        let second = CALayer::new();
        show_system_layer(&layer, &second);
        assert!(first.superlayer().is_none());
        let sublayers = unsafe { layers.content_layer.0.sublayers() }.unwrap();
        assert!(std::ptr::eq(&*sublayers.lastObject().unwrap(), &*second));

        in_transaction(|| layers.take_down_system_layer());
        assert!(second.superlayer().is_none(), "a present takes it down");
    }

    /// WaOS's picture in the root of a `CAContext`: the app's layer holds it above what it had,
    /// where the placement says, and no view may place it then.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_layer_the_app_places_lies_above_what_its_parent_had() {
        let root = flipped_view_layer();
        root.addSublayer(&CALayer::new());
        let picture = made_layer();
        let picture_layers = CoreAnimationLayer::downcast_from(&picture).unwrap();
        // A view 800 by 600 points at 2 pixels to the point, placed as its scene places a layer:
        // at its logical size, under a transform by its scale.
        let filling = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 800.0, 600.0),
            mutators: vec![Mutator::Transform(Matrix4::scale(2.0, 2.0))],
        };
        place_layer_in(&root, &picture, &filling, 2.0);
        let sublayers = unsafe { root.sublayers() }.unwrap();
        assert_eq!(sublayers.count(), 2);
        assert!(std::ptr::eq(
            &*sublayers.lastObject().unwrap(),
            &*picture_layers.clip_layer.0
        ));
        assert_eq!(
            frame_in(&picture_layers.buffer_layer.0, &root),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 600.0))
        );
        assert_eq!(
            picture_layers.content_layer.0.bounds().size,
            NSSize::new(800.0, 600.0),
            "a buffer drawn at the view's 1600 by 1200 pixels fills it, at the window's scale"
        );

        place_layer_in(&root, &picture, &filling, 2.0);
        assert_eq!(
            unsafe { root.sublayers() }.unwrap().count(),
            2,
            "placed again, it stays where it is"
        );
        assert!(!picture_layers.adopted_by(Placer::View(ViewId(0))));
    }

    fn made_layer() -> Arc<dyn CompositionLayer> {
        CoreAnimationCompositor.make_layer(true)
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

    /// A layer at `bounds` in its parent's pixels, all of it showing, under a transform that
    /// scales its own coordinates by `scale`.
    fn scaled(bounds: Rect, scale: f64) -> Placement {
        Placement {
            rect: Rect::from_ltrb(
                bounds.left / scale,
                bounds.top / scale,
                bounds.right / scale,
                bounds.bottom / scale,
            ),
            mutators: vec![Mutator::Transform(Matrix4::scale(
                scale as f32,
                scale as f32,
            ))],
        }
    }
}

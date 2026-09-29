//! Native surfaces as Core Animation layers beneath a window's content.
//!
//! A surface is a `CALayer` whose contents are the `IOSurface` its producer presented last.
//! It joins the layer tree of the winit view whose scene first places it, as a sublayer below
//! the `CAMetalLayer` wgpu draws the framework into (raw-window-metal makes that one a sublayer
//! too), so the clear rectangle the framework paints where the surface goes shows it through.
//! `docs/composition.md` has the design, and the experiment that settled each choice here.
//!
//! Three things touch a surface. Its producer presents from its own thread, in an explicit
//! transaction. The main thread places it with the window's frame. The window's display link,
//! on the main thread, releases the buffers the compositor has finished with and calls the
//! producers waiting for a vsync; a producer that asks while the link is paused has the main
//! queue restart it, so nothing of this reaches the host's event loop. The invariant a later editor must keep: a buffer's release
//! callback runs only once `IOSurfaceIsInUse` answers false for it, or when its surface is gone.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};
use std::time::Instant;

use dispatch2::{DispatchQueue, MainThreadBound};

use inset_embedder::{Buffer, Compositor, Rect, Surface, SurfacePlacement, ViewId, VsyncObserver};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_io_surface::IOSurfaceRef;
use objc2_quartz_core::{CALayer, CATransaction, kCAGravityBottomLeft};
use winit::window::Window;

use super::MacOs;
use super::ns_view;
use super::vsync::ViewDisplayLink;
use crate::os::{DisplayLink, NativeComposition, WindowSurfaces};

/// Vsyncs in a row with nobody waiting before a window's display link pauses.
const IDLE_VSYNCS: u8 = 2;

const HELD_BRIEFLY: &str = "a surface's state is never held across a panic";

impl NativeComposition for MacOs {
    type WindowSurfaces = CoreAnimationWindowSurfaces;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        Some(Arc::new(CoreAnimationCompositor))
    }
}

/// The compositor of a macOS host. Every surface is a layer of its own, so it keeps nothing.
struct CoreAnimationCompositor;

impl Compositor for CoreAnimationCompositor {
    fn open_surface(&self, view: ViewId, opaque: bool) -> Arc<dyn Surface> {
        Arc::new(CoreAnimationSurface::new(view, opaque))
    }
}

/// A surface: its layer, the buffers presented to it and not yet released, and the producers
/// waiting for a vsync.
pub(crate) struct CoreAnimationSurface {
    /// The view whose scenes may place the surface.
    view: ViewId,
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
        layer.setContentsGravity(unsafe { kCAGravityBottomLeft });
        layer.setHidden(true);
        CoreAnimationSurface {
            view,
            layer: CrossThreadLayer(layer),
            buffers: Mutex::new(PresentedBuffers::new()),
            vsync_observers: Mutex::new(Vec::new()),
            window: Mutex::new(Weak::new()),
            misplacement_reported: AtomicBool::new(false),
        }
    }

    /// The host's own surface behind a placed one; `None` for a surface no compositor of
    /// this host opened.
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

    /// Puts the layer at `index` among `parent`'s sublayers, over `rect` of the view, shown.
    /// Main thread, inside the caller's transaction.
    fn place(&self, parent: &CALayer, index: u32, rect: Rect, scale: f64) {
        let layer = &self.layer.0;
        if !is_sublayer_at(parent, layer, index) {
            parent.insertSublayer_atIndex(layer, index);
        }
        layer.setFrame(layer_frame_of(rect));
        layer.setContentsScale(scale);
        layer.setHidden(false);
    }

    fn hide(&self) {
        self.layer.0.setHidden(true);
    }

    /// From now on the surface's vsyncs come from `window`'s display link.
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
    /// The layer leaves its window, and every buffer goes back to the producer.
    fn drop(&mut self) {
        in_transaction(|| self.layer.0.removeFromSuperlayer());
        let released = self.buffers.get_mut().expect(HELD_BRIEFLY).release_all();
        notify_released(released);
    }
}

/// What one window's display link shares with the surfaces placed in it, from any thread: the
/// surfaces the last scene placed, whether the link runs, and the link itself.
pub(crate) struct PlacedSurfaces {
    surfaces_and_link: Mutex<SurfacesAndLink>,
    /// The window's display link, made on the main thread when a surface is first placed; it
    /// holds `None` where the system has no display link for a view. Any thread may hold it,
    /// only the main thread may touch the link inside, and it is dropped there too.
    display_link: OnceLock<MainThreadBound<Option<ViewDisplayLink>>>,
}

/// Under one lock, so that a producer asking for a vsync and the link deciding to pause
/// cannot miss each other.
struct SurfacesAndLink {
    surfaces: Vec<Arc<CoreAnimationSurface>>,
    link: DisplayLinkState,
}

/// What falls due at one vsync: the surfaces whose released buffers to hand back, the
/// producers to call, and whether the link runs on afterwards.
struct DueAtVsync {
    surfaces: Vec<Arc<CoreAnimationSurface>>,
    observers: Vec<Weak<dyn VsyncObserver>>,
    link_keeps_running: bool,
}

impl PlacedSurfaces {
    fn new() -> PlacedSurfaces {
        PlacedSurfaces {
            surfaces_and_link: Mutex::new(SurfacesAndLink {
                surfaces: Vec::new(),
                link: DisplayLinkState::default(),
            }),
            display_link: OnceLock::new(),
        }
    }

    /// A producer waits for a vsync: the link runs from now on, and if it was paused the main
    /// queue restarts it. Any thread.
    fn request_vsync(self: &Arc<Self>) {
        let was_paused = self.lock().link.start_if_paused();
        if was_paused {
            let placed = Arc::clone(self);
            DispatchQueue::main().exec_async(move || placed.resume_display_link());
        }
    }

    /// Makes the window's display link, paused, unless it is made already: each vsync serves
    /// the surfaces placed in the window, and the link pauses once nobody has waited for a
    /// while. Main thread.
    fn make_display_link_once(self: &Arc<Self>, window: &Window, main_thread: MainThreadMarker) {
        self.display_link.get_or_init(|| {
            let placed = Arc::downgrade(self);
            let started = ViewDisplayLink::start(
                window,
                Box::new(move || {
                    if let Some(placed) = placed.upgrade() {
                        placed.on_vsync(Instant::now());
                    }
                }),
            );
            match &started {
                Some(link) => link.set_paused(true),
                None => {
                    eprintln!("inset: no display link for a window; its surfaces get no vsync")
                }
            }
            MainThreadBound::new(started, main_thread)
        });
    }

    /// A vsync of the window's display, on the main thread: serves the placed surfaces, and
    /// pauses the link once nobody waits.
    fn on_vsync(&self, at: Instant) {
        if !self.serve_vsync(at) {
            self.set_display_link_paused(true);
        }
    }

    /// On the main queue, which a producer asked to restart the link.
    fn resume_display_link(&self) {
        self.set_display_link_paused(false);
    }

    /// Main thread.
    fn set_display_link_paused(&self, paused: bool) {
        let main_thread = MainThreadMarker::new().expect("the display link is the main thread's");
        if let Some(Some(link)) = self.display_link.get().map(|link| link.get(main_thread)) {
            link.set_paused(paused);
        }
    }

    fn is_empty(&self) -> bool {
        self.lock().surfaces.is_empty()
    }

    /// Makes `surfaces` the ones placed now, and answers the ones that were.
    fn replace_placed(
        &self,
        surfaces: Vec<Arc<CoreAnimationSurface>>,
    ) -> Vec<Arc<CoreAnimationSurface>> {
        std::mem::replace(&mut self.lock().surfaces, surfaces)
    }

    /// A vsync of the window's display: hands back the buffers the compositor has finished
    /// with, then calls every producer that waited. Answers whether the link keeps running.
    fn serve_vsync(&self, at: Instant) -> bool {
        let due = self.take_due_at_vsync();
        for surface in &due.surfaces {
            surface.release_unused_buffers();
        }
        for observer in due.observers.iter().filter_map(Weak::upgrade) {
            observer.vsync(at);
        }
        due.link_keeps_running
    }

    fn take_due_at_vsync(&self) -> DueAtVsync {
        let mut state = self.lock();
        let observers: Vec<_> = state
            .surfaces
            .iter()
            .flat_map(|surface| surface.take_vsync_observers())
            .collect();
        let link_keeps_running = state.link.keeps_running_after_vsync(!observers.is_empty());
        DueAtVsync {
            surfaces: state.surfaces.clone(),
            observers,
            link_keeps_running,
        }
    }

    fn lock(&self) -> MutexGuard<'_, SurfacesAndLink> {
        self.surfaces_and_link.lock().expect(HELD_BRIEFLY)
    }
}

/// One window's surfaces, as the main thread holds them.
pub(crate) struct CoreAnimationWindowSurfaces {
    window: Arc<Window>,
    placed: Arc<PlacedSurfaces>,
}

impl WindowSurfaces for CoreAnimationWindowSurfaces {
    fn new(window: Arc<Window>) -> Self {
        CoreAnimationWindowSurfaces {
            window,
            placed: Arc::new(PlacedSurfaces::new()),
        }
    }

    /// Hides the surfaces the scene no longer places.
    fn place(&self, placements: &[SurfacePlacement], view: ViewId) {
        let surfaces = host_surfaces_in(placements, view);
        if surfaces.is_empty() && self.placed.is_empty() {
            return;
        }
        let Some(parent) = ns_view(&self.window).and_then(|view| view.layer()) else {
            eprintln!("inset: a window's view has no layer; its surfaces are not shown");
            return;
        };
        let scale = self.window.scale_factor();
        in_transaction(|| {
            for (index, (surface, rect)) in (0u32..).zip(&surfaces) {
                surface.place(&parent, index, *rect, scale);
            }
        });
        if let Some(main_thread) = MainThreadMarker::new() {
            self.placed
                .make_display_link_once(&self.window, main_thread);
        }
        self.replace_placed(surfaces.into_iter().map(|(surface, _)| surface).collect());
    }
}

impl CoreAnimationWindowSurfaces {
    /// Makes `surfaces` the window's, hides the ones that no longer are, and asks for a vsync
    /// for producers that waited before their surface was placed.
    fn replace_placed(&self, surfaces: Vec<Arc<CoreAnimationSurface>>) {
        for surface in &surfaces {
            surface.attach_to_window(Arc::downgrade(&self.placed));
        }
        let any_waiting = surfaces
            .iter()
            .any(|surface| surface.is_waiting_for_vsync());
        let previous = self.placed.replace_placed(surfaces.clone());
        in_transaction(|| {
            for gone in previous.iter().filter(|old| !contains(&surfaces, old)) {
                gone.hide();
            }
        });
        if any_waiting {
            self.placed.request_vsync();
        }
    }
}

/// This host's surfaces a scene of `view` places, with their rectangles, in paint order.
fn host_surfaces_in(
    placements: &[SurfacePlacement],
    view: ViewId,
) -> Vec<(Arc<CoreAnimationSurface>, Rect)> {
    placements
        .iter()
        .filter_map(|placement| {
            let surface =
                CoreAnimationSurface::downcast_from(&placement.surface).or_else(|| {
                    report_foreign_surface();
                    None
                })?;
            surface
                .may_be_placed_in(view)
                .then_some((surface, placement.rect))
        })
        .collect()
}

/// Reports, once, a placed surface this host did not open.
fn report_foreign_surface() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "inset: a scene placed a surface this host's compositor did not open; it is not shown"
        );
    }
}

fn contains(surfaces: &[Arc<CoreAnimationSurface>], surface: &Arc<CoreAnimationSurface>) -> bool {
    surfaces.iter().any(|placed| Arc::ptr_eq(placed, surface))
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

/// `rect` as a layer frame in the flipped view's points.
fn layer_frame_of(rect: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.left, rect.top),
        NSSize::new(rect.width(), rect.height()),
    )
}

/// Runs `change` in a transaction of its own with implicit animations off: a layer changed
/// off the main thread must be in one, and a layer the framework moves must not animate.
fn in_transaction(change: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    change();
    CATransaction::commit();
}

/// Calls each producer's release callback: its buffer is its own again.
fn notify_released(callbacks: Vec<OnReleased>) {
    for callback in callbacks {
        callback();
    }
}

/// A `CALayer` handed between threads.
struct CrossThreadLayer(Retained<CALayer>);

// SAFETY: Core Animation guards its own state, and this module changes a layer off the main
// thread only inside an explicit transaction, as Chromium and WebKit do; the binding cannot
// say so. Measured safe for contents replacement in `experiments/2026-09-22-layer-under-a-hole`.
unsafe impl Send for CrossThreadLayer {}
unsafe impl Sync for CrossThreadLayer {}

/// A producer's buffer, handed between threads.
struct CrossThreadBuffer(Buffer);

// SAFETY: an `IOSurface` is made to be shared between threads and processes: its retain count
// and use count are atomic. The binding cannot say so.
unsafe impl Send for CrossThreadBuffer {}

impl CrossThreadBuffer {
    fn set_as_contents_of(&self, layer: &CALayer) {
        let surface: *const IOSurfaceRef = &*self.0;
        // SAFETY: a CF type is an Objective-C object, and a layer takes an IOSurface as its
        // contents; Apple documents only images there, and WebKit, Chromium and Flutter all
        // rely on it.
        unsafe { layer.setContents(Some(&*surface.cast::<AnyObject>())) };
    }
}

/// What a producer asked to run once the compositor has released its buffer.
type OnReleased = Box<dyn FnOnce() + Send + Sync>;

/// The buffers presented to a surface and not yet released: the one on screen, and the ones it
/// replaced, which the compositor may still be reading.
struct PresentedBuffers<B> {
    on_screen: Option<PresentedBuffer<B>>,
    replaced: Vec<PresentedBuffer<B>>,
}

/// A buffer presented to a surface, with what its producer asked to run once it is released.
struct PresentedBuffer<B> {
    buffer: B,
    on_released: OnReleased,
}

impl<B> PresentedBuffers<B> {
    fn new() -> PresentedBuffers<B> {
        PresentedBuffers {
            on_screen: None,
            replaced: Vec::new(),
        }
    }

    /// `buffer` is on screen now, and the one that was is replaced.
    fn present(&mut self, buffer: B, on_released: OnReleased) {
        let previous = self.on_screen.replace(PresentedBuffer {
            buffer,
            on_released,
        });
        self.replaced.extend(previous);
    }

    /// Releases the replaced buffers `in_use` no longer answers true for, and answers their
    /// callbacks for the caller to run.
    fn release_unused(&mut self, in_use: impl Fn(&B) -> bool) -> Vec<OnReleased> {
        let (still_in_use, unused): (Vec<_>, Vec<_>) = std::mem::take(&mut self.replaced)
            .into_iter()
            .partition(|presented| in_use(&presented.buffer));
        self.replaced = still_in_use;
        unused
            .into_iter()
            .map(|presented| presented.on_released)
            .collect()
    }

    /// Releases every buffer, for a surface that is going, and answers their callbacks.
    fn release_all(&mut self) -> Vec<OnReleased> {
        self.replaced
            .drain(..)
            .chain(self.on_screen.take())
            .map(|presented| presented.on_released)
            .collect()
    }
}

/// Whether a window's display link runs, and how many vsyncs in a row it has served nobody.
#[derive(Default)]
struct DisplayLinkState {
    running: bool,
    idle_vsyncs: u8,
}

impl DisplayLinkState {
    /// A producer waits for a vsync: the link runs from now on. Answers whether it was
    /// paused, so that the caller starts it.
    fn start_if_paused(&mut self) -> bool {
        self.idle_vsyncs = 0;
        !std::mem::replace(&mut self.running, true)
    }

    /// A vsync passed, having served a producer or not: answers whether the link keeps
    /// running, which it stops doing after `IDLE_VSYNCS` in a row served nobody.
    fn keeps_running_after_vsync(&mut self, served_anyone: bool) -> bool {
        if served_anyone {
            self.idle_vsyncs = 0;
            return self.running;
        }
        self.idle_vsyncs += 1;
        if self.idle_vsyncs >= IDLE_VSYNCS {
            self.running = false;
            self.idle_vsyncs = 0;
        }
        self.running
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    /// A release callback that counts itself into `count`.
    fn counting(count: &Arc<AtomicUsize>) -> OnReleased {
        let count = Arc::clone(count);
        Box::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
        })
    }

    #[test]
    fn a_buffer_is_released_only_once_it_is_replaced_and_no_longer_in_use() {
        let released = Arc::new(AtomicUsize::new(0));
        let mut buffers = PresentedBuffers::new();
        buffers.present("first", counting(&released));
        assert!(
            buffers.release_unused(|_| false).is_empty(),
            "the one on screen stays"
        );
        buffers.present("second", counting(&released));
        assert!(
            buffers
                .release_unused(|buffer| *buffer == "first")
                .is_empty(),
            "still in use"
        );
        notify_released(buffers.release_unused(|_| false));
        assert_eq!(released.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_surface_that_goes_releases_every_buffer_it_holds() {
        let released = Arc::new(AtomicUsize::new(0));
        let mut buffers = PresentedBuffers::new();
        buffers.present(1, counting(&released));
        buffers.present(2, counting(&released));
        notify_released(buffers.release_all());
        assert_eq!(released.load(Ordering::SeqCst), 2);
        assert!(buffers.release_all().is_empty());
    }

    #[test]
    fn a_display_link_starts_once_and_pauses_after_idle_vsyncs() {
        let mut link = DisplayLinkState::default();
        assert!(link.start_if_paused(), "a paused link must be started");
        assert!(!link.start_if_paused(), "a running one need not");
        assert!(link.keeps_running_after_vsync(true));
        assert!(link.keeps_running_after_vsync(false), "one idle vsync");
        assert!(!link.keeps_running_after_vsync(false), "paused after two");
        assert!(
            link.start_if_paused(),
            "a request after the pause starts it again"
        );
    }

    struct CountingObserver(AtomicUsize);

    impl VsyncObserver for CountingObserver {
        fn vsync(&self, _at: Instant) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn a_placed_surface_calls_each_waiting_producer_once_per_vsync() {
        let placed = Arc::new(PlacedSurfaces::new());
        let surface = Arc::new(CoreAnimationSurface::new(ViewId(0), true));
        let observer = Arc::new(CountingObserver(AtomicUsize::new(0)));
        let weak = Arc::downgrade(&observer) as Weak<dyn VsyncObserver>;

        surface.next_vsync(weak.clone());
        assert!(
            !placed.lock().link.running,
            "not placed: no window's link to run"
        );
        surface.attach_to_window(Arc::downgrade(&placed));
        placed.replace_placed(vec![Arc::clone(&surface)]);
        surface.next_vsync(weak);
        assert!(placed.lock().link.running, "placed: the window's link runs");

        assert!(placed.serve_vsync(Instant::now()));
        assert_eq!(
            observer.0.load(Ordering::SeqCst),
            1,
            "asked twice, called once"
        );
        assert!(placed.serve_vsync(Instant::now()));
        assert!(
            !placed.serve_vsync(Instant::now()),
            "nobody asked again: the link pauses"
        );
        assert_eq!(observer.0.load(Ordering::SeqCst), 1);
    }
}

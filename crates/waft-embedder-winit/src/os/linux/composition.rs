//! Composition layers on Wayland, as GTK 4's graphics offload puts content in subsurfaces: a layer
//! is a `wl_surface` with a `wp_viewport`, made a synchronized `wl_subsurface` of whoever places
//! it, below it: the window's own surface for a layer a view's scene places, a layer's surface for
//! a child its present places. A layer shows its producer's dmabuf as a `wl_buffer`; a buffer the
//! compositor refuses to make one of is not shown, and goes back to its producer at once. X11 has
//! no compositor a client hands buffers to, and shows no layers.
//!
//! Synchronized, a subsurface's position and its viewport's destination are applied with its
//! parent's next commit, so a scene's placements land with the view's picture: the view places its
//! layers before its swapchain presents, whose commit of the window's surface applies them
//! (`experiments/2026-11-13-linux-dmabuf-subsurfaces`, row 2). The same holds for a producer's
//! present, which is cached until the window's surface is committed again: so a present has the
//! view it reaches present again at its next vsync (`take_presented_views`), which costs the view
//! a redraw of its last scene per program frame.
//!
//! The invariants a later editor must keep: a layer has one placer at a time, and is shown only
//! where its placer's last placement or present put it; every surface's state is committed before
//! the commit of its parent that applies it; and a producer's release runs once, at the
//! `wl_buffer.release` owed to its present, or at once where nothing was shown.

use std::sync::{Arc, Mutex, OnceLock, Weak};

use waft_embedder::{
    CompositionLayer, Compositor, HardwareBuffer, LayerPicture, Placement, ViewId,
};
use wayland_client::protocol::{wl_subsurface, wl_surface};
use wayland_protocols::wp::viewporter::client::wp_viewport;
use winit::raw_window_handle::RawWindowHandle;
use winit::window::Window;

use super::dmabuf::DmabufBuffer;
use super::wayland::Wayland;
use crate::os::window_handle_layer::WindowComposition;

/// The views whose layers were presented since the loop last looked, each to present again.
static PRESENTED: Mutex<Vec<ViewId>> = Mutex::new(Vec::new());

/// What wakes the loop from a producer's thread, set once the loop is built.
static WAKE: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

pub(crate) fn wake_on_layer_presents(wake: Box<dyn Fn() + Send + Sync>) {
    let _ = WAKE.set(wake);
}

/// The views whose layers were presented since the last call.
pub(crate) fn take_presented_views() -> Vec<ViewId> {
    std::mem::take(&mut *PRESENTED.lock().unwrap())
}

/// Has `view` present again, for a layer of it a producer presented.
fn view_owes_present(view: ViewId) {
    let mut presented = PRESENTED.lock().unwrap();
    if !presented.contains(&view) {
        presented.push(view);
    }
    drop(presented);
    if let Some(wake) = WAKE.get() {
        wake();
    }
}

/// The compositor on Wayland, where the compositor offers what layers need.
pub(crate) struct WaylandCompositor {
    wayland: Arc<Wayland>,
}

impl WaylandCompositor {
    pub(crate) fn new() -> Option<WaylandCompositor> {
        Wayland::get().map(|wayland| WaylandCompositor { wayland })
    }
}

impl Compositor for WaylandCompositor {
    fn make_layer(&self, _opaque: bool) -> Arc<dyn CompositionLayer> {
        Arc::new_cyclic(|me| WaylandLayer::new(Arc::clone(&self.wayland), me.clone()))
    }
}

/// One composition layer: a surface and its viewport, a subsurface of its placer once placed.
pub(crate) struct WaylandLayer {
    wayland: Arc<Wayland>,
    /// The `Arc` this layer is in, which its children's placer names.
    me: Weak<WaylandLayer>,
    surface: wl_surface::WlSurface,
    viewport: wp_viewport::WpViewport,
    state: Mutex<LayerState>,
}

#[derive(Default)]
struct LayerState {
    /// The subsurface under the placer, and the placer.
    placed: Option<(wl_subsurface::WlSubsurface, Placer)>,
    /// How many of the surface's logical units one pixel of its buffer spans, from its placement.
    units_per_pixel: f64,
    /// The pixels a producer draws to one unit of a layer, the view's device pixel ratio.
    pixel_ratio: f64,
    buffer: Option<Arc<dyn HardwareBuffer>>,
    children: Vec<Arc<WaylandLayer>>,
}

/// Who placed a layer.
#[derive(Clone)]
enum Placer {
    View(ViewId),
    Layer(Weak<WaylandLayer>),
}

impl Placer {
    fn is(&self, other: &Placer) -> bool {
        match (self, other) {
            (Placer::View(a), Placer::View(b)) => a == b,
            (Placer::Layer(a), Placer::Layer(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
}

/// Where a layer goes in its parent, in the parent's logical units, and what of its buffer shows.
#[derive(Debug, PartialEq)]
struct Geometry {
    position: [i32; 2],
    /// The viewport's source, in the buffer's pixels: what of the buffer shows.
    source: [f64; 4],
    /// The viewport's destination, in logical units.
    destination: [i32; 2],
    units_per_pixel: f64,
}

/// Where `placement`, in its parent's pixels, puts a layer whose buffer is `buffer_size` pixels,
/// in a parent whose pixel spans `parent_units_per_pixel` logical units, drawn at `pixel_ratio`:
/// the box around it moved by integers and scaled, cut to the bounds of every clip above it. `None`
/// where nothing of it shows.
fn geometry(
    placement: &Placement,
    pixel_ratio: f64,
    parent_units_per_pixel: f64,
) -> Option<Geometry> {
    let scale = placement.scale() / pixel_ratio;
    let visible = placement.visible_bounds();
    if placement.opacity() <= 0.0 || !(scale.is_finite() && scale > 0.0) || visible.is_empty() {
        return None;
    }
    let bounds = placement.bounds();
    let local = |parent: f64, origin: f64| (parent - origin) / scale;
    let source = [
        local(visible.left, bounds.left),
        local(visible.top, bounds.top),
        (visible.right - visible.left) / scale,
        (visible.bottom - visible.top) / scale,
    ];
    let units = |pixels: f64| (pixels * parent_units_per_pixel).round() as i32;
    Some(Geometry {
        position: [units(visible.left), units(visible.top)],
        source,
        destination: [
            units(visible.right - visible.left).max(1),
            units(visible.bottom - visible.top).max(1),
        ],
        units_per_pixel: scale * parent_units_per_pixel,
    })
}

impl WaylandLayer {
    fn new(wayland: Arc<Wayland>, me: Weak<WaylandLayer>) -> WaylandLayer {
        let surface = wayland.compositor.create_surface(wayland.queue(), ());
        let viewport = wayland
            .viewporter
            .get_viewport(&surface, wayland.queue(), ());
        // The layer takes no input: the window's own surface does, for the view to forward.
        let nothing = wayland.compositor.create_region(wayland.queue(), ());
        surface.set_input_region(Some(&nothing));
        nothing.destroy();
        WaylandLayer {
            wayland,
            me,
            surface,
            viewport,
            state: Mutex::new(LayerState {
                units_per_pixel: 1.0,
                pixel_ratio: 1.0,
                ..LayerState::default()
            }),
        }
    }

    /// Places the layer under `parent`, a surface of `placer`, at `geometry`, below `parent`
    /// and above the siblings placed before it, its buffers drawn at `pixel_ratio`; a layer another
    /// placer holds is not moved.
    fn place_in(
        &self,
        parent: &wl_surface::WlSurface,
        placer: Placer,
        geometry: &Geometry,
        pixel_ratio: f64,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        let reuse = matches!(&state.placed, Some((_, held)) if held.is(&placer));
        if !reuse {
            if state.placed.is_some() {
                return false;
            }
            let subsurface = self.wayland.subcompositor.get_subsurface(
                &self.surface,
                parent,
                self.wayland.queue(),
                (),
            );
            subsurface.set_sync();
            state.placed = Some((subsurface, placer));
        }
        let (subsurface, _) = state.placed.as_ref().expect("placed above");
        subsurface.set_position(geometry.position[0], geometry.position[1]);
        subsurface.place_below(parent);
        let [x, y, width, height] = geometry.source;
        self.viewport.set_source(x, y, width, height);
        self.viewport
            .set_destination(geometry.destination[0], geometry.destination[1]);
        state.units_per_pixel = geometry.units_per_pixel;
        state.pixel_ratio = pixel_ratio;
        drop(state);
        self.surface.commit();
        true
    }

    /// Takes the layer off its placer, which shows it no more.
    fn unplace(&self) {
        if let Some((subsurface, _)) = self.state.lock().unwrap().placed.take() {
            subsurface.destroy();
        }
    }

    /// The view the layer's top-most placer is, if a view's scene placed it.
    fn view(&self) -> Option<ViewId> {
        let placer = self.state.lock().unwrap().placed.as_ref()?.1.clone();
        match placer {
            Placer::View(view) => Some(view),
            Placer::Layer(parent) => parent.upgrade()?.view(),
        }
    }

    /// The children `children` asks for placed beneath this layer's buffer, and the ones it
    /// no longer names let go.
    fn place_children(self: &Arc<Self>, children: Vec<(Arc<dyn CompositionLayer>, Placement)>) {
        let (units_per_pixel, pixel_ratio) = {
            let state = self.state.lock().unwrap();
            (state.units_per_pixel, state.pixel_ratio)
        };
        let placer = Placer::Layer(Arc::downgrade(self));
        let mut placed = Vec::new();
        for (child, placement) in children {
            let Some(child) = as_wayland(child) else {
                eprintln!("waft: a layer of another host is not shown as a child");
                continue;
            };
            let Some(geometry) = geometry(&placement, pixel_ratio, units_per_pixel) else {
                child.unplace();
                continue;
            };
            if child.place_in(&self.surface, placer.clone(), &geometry, pixel_ratio) {
                placed.push(child);
            } else {
                eprintln!("waft: a layer another placer holds is not shown here");
            }
        }
        let previous = std::mem::replace(&mut self.state.lock().unwrap().children, placed.clone());
        for gone in previous {
            if !placed.iter().any(|kept| Arc::ptr_eq(kept, &gone)) {
                gone.unplace();
            }
        }
    }
}

fn as_wayland(layer: Arc<dyn CompositionLayer>) -> Option<Arc<WaylandLayer>> {
    let any: Arc<dyn std::any::Any + Send + Sync> = layer;
    any.downcast::<WaylandLayer>().ok()
}

impl CompositionLayer for WaylandLayer {
    fn present(
        &self,
        buffer: Arc<dyn HardwareBuffer>,
        children: Vec<(Arc<dyn CompositionLayer>, Placement)>,
        released: Box<dyn FnOnce() + Send + Sync>,
    ) {
        let any: &dyn std::any::Any = &*buffer;
        let made = any
            .downcast_ref::<DmabufBuffer>()
            .and_then(|dmabuf| dmabuf.wl_buffer(&self.wayland));
        let Some((wl_buffer, data)) = made else {
            released();
            return;
        };
        if let Some(this) = self.me.upgrade() {
            this.place_children(children);
        }
        data.release_with(released);
        self.surface.attach(Some(&wl_buffer), 0, 0);
        self.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        self.surface.commit();
        self.wayland.flush();
        self.state.lock().unwrap().buffer = Some(buffer);
        if let Some(view) = self.view() {
            view_owes_present(view);
        }
    }

    fn current_picture(&self) -> Option<LayerPicture> {
        self.state
            .lock()
            .unwrap()
            .buffer
            .clone()
            .map(LayerPicture::Buffer)
    }
}

/// A window's share of the compositor on Wayland: the window's own surface, which the layers a
/// scene of its view places are subsurfaces of, and the layers placed last.
pub(crate) struct LinuxComposition {
    window: Arc<dyn Window>,
    placed: Mutex<Vec<Arc<WaylandLayer>>>,
}

impl WindowComposition for LinuxComposition {
    fn new(window: Arc<dyn Window>) -> Self {
        LinuxComposition {
            window,
            placed: Mutex::new(Vec::new()),
        }
    }

    fn place<'a>(
        &self,
        placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        view: ViewId,
    ) {
        let Some(wayland) = Wayland::get() else {
            return;
        };
        let Some(root) = self.surface(&wayland) else {
            return;
        };
        let scale = self.window.scale_factor();
        let mut placed = Vec::new();
        for (layer, placement) in placements {
            let Some(layer) = as_wayland(Arc::clone(layer)) else {
                eprintln!("waft: a layer of another host is not shown");
                continue;
            };
            let Some(geometry) = geometry(placement, scale, 1.0 / scale) else {
                layer.unplace();
                continue;
            };
            if layer.place_in(&root, Placer::View(view), &geometry, scale) {
                placed.push(layer);
            } else {
                eprintln!("waft: a layer another view places is not shown here");
            }
        }
        let previous = std::mem::replace(&mut *self.placed.lock().unwrap(), placed.clone());
        for gone in previous {
            if !placed.iter().any(|kept| Arc::ptr_eq(kept, &gone)) {
                gone.unplace();
            }
        }
        wayland.flush();
    }
}

impl LinuxComposition {
    /// The window's own `wl_surface`, where the window is Wayland's.
    fn surface(&self, wayland: &Wayland) -> Option<wl_surface::WlSurface> {
        let handle = self.window.rwh_06_window_handle().window_handle().ok()?;
        let RawWindowHandle::Wayland(handle) = handle.as_raw() else {
            return None;
        };
        wayland.foreign_surface(handle.surface.as_ptr())
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Matrix4, Mutator, Rect};

    use super::*;

    #[test]
    fn a_layer_drawn_at_the_views_ratio_fills_its_box_in_logical_units() {
        let placement = Placement {
            rect: Rect::from_ltwh(20.0, 40.0, 320.0, 180.0),
            mutators: vec![Mutator::Transform(Matrix4::scale(2.0, 2.0))],
        };
        let geometry = geometry(&placement, 2.0, 0.5).expect("it shows");
        assert_eq!(geometry.position, [20, 40]);
        assert_eq!(geometry.source, [0.0, 0.0, 640.0, 360.0]);
        assert_eq!(geometry.destination, [320, 180]);
        assert_eq!(geometry.units_per_pixel, 0.5);
    }

    #[test]
    fn a_clip_cuts_the_source_and_moves_the_layer_to_what_shows() {
        let placement = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 400.0, 400.0),
            mutators: vec![
                Mutator::Transform(Matrix4::scale(2.0, 2.0)),
                Mutator::ClipRect(Rect::from_ltwh(50.0, 50.0, 100.0, 100.0)),
                Mutator::Transform(Matrix4::scale(0.5, 0.5)),
            ],
        };
        let geometry = geometry(&placement, 2.0, 0.5).expect("it shows");
        assert_eq!(geometry.position, [50, 50]);
        assert_eq!(geometry.source, [200.0, 200.0, 400.0, 400.0]);
        assert_eq!(geometry.destination, [100, 100]);
    }

    #[test]
    fn a_layer_nothing_lets_show_is_not_placed() {
        let faded = Placement {
            rect: Rect::from_ltwh(0.0, 0.0, 10.0, 10.0),
            mutators: vec![Mutator::Opacity(0)],
        };
        assert_eq!(geometry(&faded, 1.0, 1.0), None);
    }
}

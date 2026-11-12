//! Composition layers as SurfaceFlinger's surface controls beneath the view's picture.
//!
//! A composition layer is two surface controls. Its container is the layer placed in its parent:
//! moved, scaled and cropped by its placement (`layer_geometry`), which crops its children with
//! it, and holding, as siblings, its children's containers (z 0 to n − 1) and on top its buffer's
//! surface control (z n), whose buffer is the `AHardwareBuffer` its producer presented last. Child
//! beneath buffer is among siblings, the one order the NDK documents (`setZOrder`). A layer is a
//! record of the host's first: a surface control needs a parent, so its two are made at its first
//! placement, under the placer, and reparented after, to another parent or to none, which takes it
//! off screen. The view's own picture is such a layer (`view_layer`), placed filling the window's
//! surface control, its children the composition layers the view's scene places, so the picture
//! and the layers beneath it land in one transaction, as Waft's contract asks and as Chromium's
//! `SurfaceControl` output does. Every length is in physical pixels from the top-left, as a
//! placement's are. The experiment `experiments/2026-11-11-android-surface-control` settled the
//! choices here.
//!
//! The module holds the compositor and the composition layer; `layer_geometry` where a placement
//! puts a layer's container, `buffers` what a layer was presented and when each goes back,
//! `transaction` one frame of the tree and the buffers it replaces, `releases` the thread that
//! waits on release fences, `view_layer` the view's own picture, `frames_in_flight` how far the
//! view's frames run ahead of the GPU, and `window_surface` the window's own surface beneath it,
//! whose buffer Android sizes the window's input by. A producer's buffer is presented with no
//! acquire fence, its GPU work done, as Waft's contract asks; the view's own carries its frame's
//! fence (`frame_fence`).
//!
//! Every present, placement and letting go is built and applied as one transaction under the
//! compositor's lock, from any thread: a producer presents from its own, a view from the loop's.
//! The invariants a later editor must keep: a replaced buffer's release runs only once the release
//! fence the system hands over for it has signalled, or when its layer goes; and a layer has one
//! placer at a time, from the placement that adopts it until that placer lets it go. A window's
//! placer names its view, so a layer two views' scenes place is one view's, and is reported for
//! the other.

mod buffers;
mod frames_in_flight;
mod layer_geometry;
mod releases;
mod transaction;
mod view_layer;
mod window_surface;

use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use waft_embedder::{
    CompositionLayer, Compositor, HardwareBuffer, LayerPicture, Placement, ViewId,
};

use crate::hardware_buffers::AndroidHardwareBuffer;
use crate::surface_control::{Geometry, SurfaceControl, Transaction};

use buffers::PresentedBuffers;
use releases::{OnReleased, Release, Releases};
use transaction::{LayerTransaction, Transactions};

pub(crate) use view_layer::SurfaceControlViewLayer;

/// The compositor of Android: every layer it makes shares one lock to build transactions under
/// and one thread that waits on release fences.
pub(crate) struct SurfaceControlCompositor {
    transactions: Arc<Transactions>,
}

impl SurfaceControlCompositor {
    /// The compositor, its fence thread started; `None` where the system starts none.
    pub(crate) fn new() -> Option<SurfaceControlCompositor> {
        Some(SurfaceControlCompositor {
            transactions: Arc::new(Transactions::new(Releases::start()?)),
        })
    }

    fn layer(&self, opaque: bool) -> Arc<SurfaceControlLayer> {
        Arc::new(SurfaceControlLayer::new(
            opaque,
            Arc::clone(&self.transactions),
        ))
    }
}

impl Compositor for SurfaceControlCompositor {
    fn make_layer(&self, opaque: bool) -> Arc<dyn CompositionLayer> {
        self.layer(opaque)
    }
}

/// Who places a layer: the presents of one layer, by its number, or a window, the picture of the
/// view named in its window's surface control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placer {
    Layer(u64),
    Window(ViewId),
}

/// A composition layer: who places it, and its state, which every transaction changes under its
/// lock, its parent's taken first.
pub(crate) struct SurfaceControlLayer {
    /// What a child names its parent by.
    number: u64,
    /// Whether it was made opaque: its buffer is shown so while no children lie beneath it.
    opaque: bool,
    transactions: Arc<Transactions>,
    /// The placer holding the layer, from the placement that adopted it until it lets the layer
    /// go; `None` while nothing places it.
    placer: Mutex<Option<Placer>>,
    /// Whether a placement by a second placer was reported already.
    misplacement_reported: AtomicBool,
    state: Mutex<LayerState>,
}

/// What a layer shows and where.
struct LayerState {
    /// Its surface controls, made at its first placement.
    surfaces: Option<LayerSurfaces>,
    buffers: PresentedBuffers,
    /// The children its last present showed, bottom-most first.
    children: Vec<(Arc<SurfaceControlLayer>, Placement)>,
    /// The device pixel ratio the layer was last placed at, which its children are laid out at;
    /// `None` until it is placed.
    pixel_ratio: Option<f64>,
}

/// A layer's surface controls: the container its placement moves, and its buffer's on top of its
/// children in it; and the number of the surface control the container lies in, none once its
/// placer let it go, which took it out of its parent.
struct LayerSurfaces {
    container: Arc<SurfaceControl>,
    buffer: Arc<SurfaceControl>,
    parent: Option<u64>,
}

impl SurfaceControlLayer {
    fn new(opaque: bool, transactions: Arc<Transactions>) -> SurfaceControlLayer {
        static NEXT_NUMBER: AtomicU64 = AtomicU64::new(0);
        SurfaceControlLayer {
            number: NEXT_NUMBER.fetch_add(1, Ordering::Relaxed),
            opaque,
            transactions,
            placer: Mutex::new(None),
            misplacement_reported: AtomicBool::new(false),
            state: Mutex::new(LayerState {
                surfaces: None,
                buffers: PresentedBuffers::new(),
                children: Vec::new(),
                pixel_ratio: None,
            }),
        }
    }

    /// A layer as this one was made, with nothing presented and placed nowhere, to take the place
    /// of this one where it is dropped for what it holds.
    fn successor(&self) -> Arc<SurfaceControlLayer> {
        Arc::new(SurfaceControlLayer::new(
            self.opaque,
            Arc::clone(&self.transactions),
        ))
    }

    /// The host's own layer behind a placed one; `None` for a layer no compositor of this host
    /// made.
    fn downcast_from(layer: &Arc<dyn CompositionLayer>) -> Option<Arc<SurfaceControlLayer>> {
        let layer = Arc::clone(layer);
        let any: Arc<dyn std::any::Any + Send + Sync> = layer;
        any.downcast().ok()
    }

    fn state(&self) -> MutexGuard<'_, LayerState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether `placer` holds the layer, adopting it when nothing does, and reporting once when
    /// another placer does.
    fn adopted_by(&self, placer: Placer) -> bool {
        let mut holder = self.placer.lock().unwrap_or_else(PoisonError::into_inner);
        if *holder.get_or_insert(placer) == placer {
            return true;
        }
        if !self.misplacement_reported.swap(true, Ordering::Relaxed) {
            crate::log::warn_to_log!(
                "a composition layer {:?} holds was placed by {placer:?}; it is not shown there",
                *holder
            );
        }
        false
    }

    /// Its placer lets it go: it is taken off screen until a placer adopts it again.
    fn let_go(&self, transaction: &mut LayerTransaction) {
        *self.placer.lock().unwrap_or_else(PoisonError::into_inner) = None;
        if let Some(surfaces) = &mut self.state().surfaces {
            transaction.transaction.reparent(&surfaces.container, None);
            surfaces.parent = None;
        }
    }

    /// Shows the layer in `parent` at `z` among its siblings, where `geometry` says, or hides it
    /// where nothing of it shows, its children laid out at `pixel_ratio`; its surface controls
    /// are made under `parent` if it has none, and moved into it if they lie elsewhere.
    fn show_in(
        &self,
        transaction: &mut LayerTransaction,
        parent: &SurfaceControl,
        z: i32,
        geometry: Option<Geometry>,
        pixel_ratio: f64,
    ) {
        let mut state = self.state();
        let Some(new) = state.surfaces_in(parent, transaction) else {
            return;
        };
        let Some(surfaces) = &state.surfaces else {
            return;
        };
        let container = Arc::clone(&surfaces.container);
        let tx = &mut transaction.transaction;
        tx.set_z_order(&container, z);
        tx.set_visible(&container, geometry.is_some());
        if let Some(geometry) = &geometry {
            tx.set_geometry(&container, geometry);
        }
        if new || state.pixel_ratio != Some(pixel_ratio) {
            state.pixel_ratio = Some(pixel_ratio);
            state.lay_out(transaction, self.opaque);
        }
    }

    /// Shows `buffer`, once `acquire_fence` signals where there is one, and places `children`
    /// beneath it in `transaction`, letting go of the children no longer named; the buffer it
    /// replaces goes back to its producer once the compositor is done with it.
    fn present_in(
        &self,
        transaction: &mut LayerTransaction,
        buffer: AndroidHardwareBuffer,
        acquire_fence: Option<OwnedFd>,
        children: Vec<(Arc<SurfaceControlLayer>, Placement)>,
        released: OnReleased,
    ) {
        let release = Release::new(Arc::new(buffer.clone()), released);
        let mut state = self.state();
        let replaced = state.buffers.present(buffer, acquire_fence, release);
        let previous_children = std::mem::replace(
            &mut state.children,
            adopted_by(children, Placer::Layer(self.number)),
        );
        for (gone, _) in previous_children
            .iter()
            .filter(|(old, _)| !state.names(old))
        {
            gone.let_go(transaction);
        }
        transaction.keep(previous_children.into_iter().map(|(layer, _)| layer));
        let Some(surfaces) = &state.surfaces else {
            if let Some(replaced) = replaced {
                transaction.release_unshown(replaced.release);
            }
            return;
        };
        let buffer_surface = Arc::clone(&surfaces.buffer);
        match replaced {
            Some(replaced) if replaced.set => {
                transaction.replaces(&buffer_surface, replaced.release)
            }
            Some(replaced) => transaction.release_unshown(replaced.release),
            None => {}
        }
        state.lay_out(transaction, self.opaque);
    }

    /// The buffer presented last, held as the layer holds it.
    fn presented_picture(&self) -> Option<LayerPicture> {
        let state = self.state();
        let shown = state.buffers.on_screen()?;
        Some(LayerPicture::Buffer(Arc::new(shown.buffer.clone())))
    }
}

impl LayerState {
    /// Makes the layer's surface controls under `parent` if it has none, or moves its container
    /// into `parent` if it lies elsewhere or nowhere, as one let go and placed again does, and
    /// answers whether they are new; `None` where the system made none.
    fn surfaces_in(
        &mut self,
        parent: &SurfaceControl,
        transaction: &mut LayerTransaction,
    ) -> Option<bool> {
        if let Some(surfaces) = &mut self.surfaces {
            if surfaces.parent != Some(parent.number()) {
                transaction
                    .transaction
                    .reparent(&surfaces.container, Some(parent));
                surfaces.parent = Some(parent.number());
            }
            return Some(false);
        }
        let container = SurfaceControl::under(parent, c"waft composition layer")?;
        let buffer = SurfaceControl::under(&container, c"waft composition layer's buffer")?;
        self.surfaces = Some(LayerSurfaces {
            container: Arc::new(container),
            buffer: Arc::new(buffer),
            parent: Some(parent.number()),
        });
        Some(true)
    }

    /// Whether the last present names `layer` among its children.
    fn names(&self, layer: &Arc<SurfaceControlLayer>) -> bool {
        self.children
            .iter()
            .any(|(child, _)| Arc::ptr_eq(child, layer))
    }

    /// Sets the buffer on screen where no transaction has, and lays the children out beneath it
    /// in the container, once the layer has been placed; the buffer is shown opaque while the
    /// layer is opaque and has no children, which would never show beneath it.
    fn lay_out(&mut self, transaction: &mut LayerTransaction, opaque: bool) {
        let (Some(surfaces), Some(pixel_ratio)) = (&self.surfaces, self.pixel_ratio) else {
            return;
        };
        let (container, buffer_surface) = (
            Arc::clone(&surfaces.container),
            Arc::clone(&surfaces.buffer),
        );
        if let Some(unset) = self.buffers.unset() {
            let acquire_fence = unset.acquire_fence.take();
            transaction.transaction.set_buffer(
                &buffer_surface,
                unset.buffer.buffer(),
                acquire_fence,
            );
            unset.set = true;
        }
        for ((child, placement), z) in self.children.iter().zip(0..) {
            let geometry = layer_geometry::of(placement, pixel_ratio);
            child.show_in(transaction, &container, z, geometry, pixel_ratio);
        }
        let tx = &mut transaction.transaction;
        tx.set_z_order(&buffer_surface, self.children.len() as i32);
        tx.set_opaque(&buffer_surface, opaque && self.children.is_empty());
    }
}

impl CompositionLayer for SurfaceControlLayer {
    /// Shows the buffer, its GPU work done, so with no acquire fence, and places the children in
    /// one transaction. A buffer another host handed out is not shown, and is reported.
    fn present(
        &self,
        buffer: Arc<dyn HardwareBuffer>,
        children: Vec<(Arc<dyn CompositionLayer>, Placement)>,
        released: Box<dyn FnOnce() + Send + Sync>,
    ) {
        let buffer: &dyn std::any::Any = &*buffer;
        let Some(buffer) = buffer.downcast_ref::<AndroidHardwareBuffer>() else {
            report_foreign_buffer();
            released();
            return;
        };
        let children = own_layers(&children);
        self.transactions.transact(|transaction| {
            self.present_in(transaction, buffer.clone(), None, children, released);
        });
    }

    /// The buffer presented last: a surface control reads nothing back.
    fn current_picture(&self) -> Option<LayerPicture> {
        self.presented_picture()
    }
}

impl Drop for SurfaceControlLayer {
    /// The container leaves its parent, its children with it, which are let go, and every buffer
    /// goes back to its producer. Applied on its own: nothing that builds a transaction holds the
    /// layer any longer.
    fn drop(&mut self) {
        let state = self.state.get_mut().unwrap_or_else(PoisonError::into_inner);
        if let Some(surfaces) = state.surfaces.take() {
            let mut transaction = Transaction::new();
            transaction.reparent(&surfaces.container, None);
            transaction.apply();
        }
        for (child, _) in state.children.drain(..) {
            *child.placer.lock().unwrap_or_else(PoisonError::into_inner) = None;
        }
        for release in state.buffers.release_all() {
            release.run();
        }
    }
}

/// The layers among `placements` `placer` holds, adopting those nothing holds, in paint order.
fn adopted_by(
    placements: Vec<(Arc<SurfaceControlLayer>, Placement)>,
    placer: Placer,
) -> Vec<(Arc<SurfaceControlLayer>, Placement)> {
    placements
        .into_iter()
        .filter(|(layer, _)| layer.adopted_by(placer))
        .collect()
}

/// This host's layers among `placements`, with their placements, in paint order.
fn own_layers(
    placements: &[(Arc<dyn CompositionLayer>, Placement)],
) -> Vec<(Arc<SurfaceControlLayer>, Placement)> {
    placements
        .iter()
        .filter_map(|(layer, placement)| {
            let Some(layer) = SurfaceControlLayer::downcast_from(layer) else {
                report_foreign_layer();
                return None;
            };
            Some((layer, placement.clone()))
        })
        .collect()
}

/// Says, once, that a buffer no host of this system handed out was presented, and is not shown.
fn report_foreign_buffer() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        crate::log::warn_to_log!(
            "a buffer this host did not hand out was presented to a layer; not shown"
        );
    }
}

/// Says, once, that a placed layer this host did not make is not shown.
fn report_foreign_layer() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        crate::log::warn_to_log!(
            "a composition layer this host did not make was placed; it is not shown"
        );
    }
}

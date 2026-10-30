//! Native surfaces the OS compositor shows beside the framework's picture: the capability a
//! host with a compositor hands out, and the surface a producer feeds from a thread of its
//! own. The Core Animation host's `os/apple/composition` module, in `inset-embedder-winit`, says
//! why it is shaped as it is. A producer that paces by the screen's refreshes waits on the
//! platform's [`VsyncWaiter`](crate::VsyncWaiter).
//!
//! Surfaces make one tree rooted at each view, as every compositor's layers do (Core Animation's
//! sublayers, Wayland's subsurfaces, Android's `SurfaceControl` parents, DirectComposition's
//! visuals), with one rule at every level: a surface is opened under a parent
//! ([`SurfaceParent::open_child`]), and placed when that parent's next frame is applied. A view's
//! frame is its [`Scene`](crate::Scene), whose [`slices`](crate::slices) place the surfaces opened
//! under the view's root ([`Compositor::root`]); a surface's frame is its [`SurfaceFrame`], whose
//! `children` place the surfaces opened under it, in its buffer's pixels, beneath its buffer.
//! Both are [`SurfacePlacement`]s, and the host lands each frame in the one transaction its
//! compositor has for it (a `CATransaction`, a parent's `wl_surface.commit`,
//! `IDCompositionDevice::Commit`, a `SurfaceControl.Transaction`). A program whose picture is
//! itself a surface in another app's window opens its own surfaces under that picture so, and
//! the placement of the picture moves, clips and scales the whole group as one.
//!
//! The invariant: a surface is on screen only where its parent's last frame placed it, and the
//! host, not the framework, owns the native object behind it.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use crate::{SurfacePlacement, ViewId};

/// The host's compositor, as an app reaches it.
pub trait Compositor: Send + Sync {
    /// `view` as the parent of the surfaces opened under it, which its frames' scenes place.
    fn root(&self, view: ViewId) -> Arc<dyn SurfaceParent>;
}

/// What surfaces open under: a view's root, or another surface. Every method may be called from
/// any thread.
pub trait SurfaceParent: Send + Sync {
    /// A surface under this parent, fed by a producer from any thread, and shown wherever the
    /// parent's frames place it: a view's scenes, a surface's presents beneath its buffer, never
    /// another parent's. `opaque` says whether its buffers carry alpha the compositor must blend;
    /// a surface shows its children through the clear pixels of its own buffer, so one with
    /// children is opened not opaque. The surface goes when its last handle does: its producer's,
    /// and those of the frames that place it; and it is gone from the screen with its parent.
    fn open_child(&self, opaque: bool) -> Arc<dyn Surface>;
}

/// One surface, and the parent of the surfaces opened under it. Every method may be called from
/// any thread.
///
/// The handle is the surface's identity: a frame places the `Arc` itself, so only a surface
/// that exists can be placed, and it lives while anything shows it. A host gets its own
/// type back from a placed surface by downcasting through `Any`.
pub trait Surface: SurfaceParent + Any + Send + Sync {
    /// Shows `frame` from the compositor's next frame on: its buffer, and its children beneath
    /// it where they say, in one step. A child is on screen only where the last present placed
    /// it.
    ///
    /// Returns once the host holds the buffer, not once it is on screen. `released` runs, on any
    /// thread, when the compositor has finished with the buffer and the producer may draw into
    /// it again. On Apple platforms the host learns that from `IOSurfaceIsInUse` at the
    /// surface's next present, or when the surface goes, so a producer there never waits for a
    /// buffer to come back: it draws into one it knows is free, or a new one.
    fn present(&self, frame: SurfaceFrame, released: Box<dyn FnOnce() + Send + Sync>);
}

/// What one present of a surface shows: a buffer, and the surface's children placed beneath it,
/// bottom-most first, as a [`Scene`](crate::Scene) is what one frame of a view shows. What a
/// later present needs beyond these, such as the part of the buffer that changed, joins them
/// here.
pub struct SurfaceFrame {
    /// The picture, which fills the surface.
    pub buffer: Buffer,
    /// Surfaces opened under the presented one (`SurfaceParent::open_child`), each placed as a
    /// scene places one in its view, in the buffer's pixels instead of the view's: the buffer's own
    /// placement moves, clips and scales them with it.
    pub children: Vec<SurfacePlacement>,
}

impl SurfaceFrame {
    /// A frame showing `buffer`, placing no child: what a surface without children presents.
    pub fn new(buffer: Buffer) -> SurfaceFrame {
        SurfaceFrame {
            buffer,
            children: Vec::new(),
        }
    }

    /// The same frame placing a child too, above the children placed before it: a program's
    /// placements come whole from its own scene's slices, with what the containers above each
    /// of its surfaces do to it.
    pub fn child(mut self, placement: SurfacePlacement) -> SurfaceFrame {
        self.children.push(placement);
        self
    }
}

impl fmt::Debug for SurfaceFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceFrame")
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

/// What a producer draws into and hands over: the platform's own type.
#[cfg(target_vendor = "apple")]
pub type Buffer = objc2_core_foundation::CFRetained<objc2_io_surface::IOSurfaceRef>;

/// A platform with no host that composites: no buffer can exist, so
/// [`Surface::present`] cannot be called, and [`Platform::compositor`](crate::Platform::compositor)
/// answers `None`.
#[cfg(not(target_vendor = "apple"))]
pub enum Buffer {}

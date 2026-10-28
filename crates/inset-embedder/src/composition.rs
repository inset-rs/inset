//! Native surfaces the OS compositor shows beside the framework's picture: the capability a
//! host with a compositor hands out, and the surface a producer feeds from a thread of its
//! own. The Core Animation host's `os/apple/composition` module, in `inset-embedder-winit`, says
//! why it is shaped as it is. A producer that paces by the screen's refreshes waits on the
//! platform's [`VsyncWaiter`](crate::VsyncWaiter).
//!
//! A surface may hold child surfaces beneath its own buffer, which its own presents place, in
//! its buffer's pixels: a program whose picture is itself a surface in another app's window
//! places its own surfaces under that picture so, and the placement of the parent moves, clips
//! and scales the whole group as one.
//!
//! The invariant: a surface is on screen only where a frame's [`Scene`](crate::Scene) places
//! it, or its parent's last present does, and the host, not the framework, owns the native
//! object behind it.

use std::any::Any;
use std::sync::Arc;

use crate::{SurfacePlacement, ViewId};

/// The host's compositor, as an app reaches it.
pub trait Compositor: Send + Sync {
    /// A surface of `view`'s, shown wherever a frame's scene places it, fed by a producer
    /// from any thread. `opaque` says whether its buffers carry alpha the compositor must
    /// blend. The surface goes when its last handle does: the producer's, and those of the
    /// frames that place it.
    fn open_surface(&self, view: ViewId, opaque: bool) -> Arc<dyn Surface>;
}

/// One surface. Every method may be called from any thread.
///
/// The handle is the surface's identity: a scene places the `Arc` itself, so only a surface
/// that exists can be placed, and it lives while anything shows it. A host gets its own
/// type back from a placed surface by downcasting through `Any`.
pub trait Surface: Any + Send + Sync {
    /// Shows `buffer` from the compositor's next frame on, with `children`, surfaces this one
    /// opened, beneath it where they say, in the same step: a child is on screen only where the
    /// last present placed it. Each child's placement is in the buffer's pixels, as a scene's
    /// are in its view's, and the buffer's own placement moves, clips and scales them with it.
    ///
    /// Returns once the host holds the buffer, not once it is on screen. `released` runs, on any
    /// thread, when the compositor has finished with the buffer and the producer may draw into
    /// it again. On Apple platforms the host learns that from `IOSurfaceIsInUse` at the
    /// surface's next present, or when the surface goes, so a producer there never waits for a
    /// buffer to come back: it draws into one it knows is free, or a new one.
    fn present(
        &self,
        buffer: Buffer,
        children: &[SurfacePlacement],
        released: Box<dyn FnOnce() + Send + Sync>,
    );

    /// A surface beneath this one's buffer, shown wherever this one's presents place it, never in
    /// a view's scene. `opaque` says whether its buffers carry alpha the compositor must blend;
    /// a parent shows a child through the clear pixels of its own buffer, so a parent with
    /// children is opened not opaque. The child goes when its last handle does, its parent's
    /// last present among them, and it is gone from the screen with its parent.
    fn open_child(&self, opaque: bool) -> Arc<dyn Surface>;
}

/// What a producer draws into and hands over: the platform's own type.
#[cfg(target_vendor = "apple")]
pub type Buffer = objc2_core_foundation::CFRetained<objc2_io_surface::IOSurfaceRef>;

/// A platform with no host that composites: no buffer can exist, so
/// [`Surface::present`] cannot be called, and [`Platform::compositor`](crate::Platform::compositor)
/// answers `None`.
#[cfg(not(target_vendor = "apple"))]
pub enum Buffer {}

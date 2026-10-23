//! Native surfaces the OS compositor shows beside the framework's picture: the capability a
//! host with a compositor hands out, and the surface a producer feeds from a thread of its
//! own. The macOS host's `os/macos/composition` module, in `inset-embedder-winit`, says why it is
//! shaped as it is. A producer that paces by the screen's refreshes waits on the platform's
//! [`VsyncWaiter`](crate::VsyncWaiter).
//!
//! The invariant: a surface is on screen only where a frame's [`Scene`](crate::Scene) places
//! it, and the host, not the framework, owns the native object behind it.

use std::any::Any;
use std::sync::Arc;

use crate::ViewId;

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
    /// Shows `buffer` from the compositor's next frame on. Returns once the host holds the
    /// buffer, not once it is on screen. `released` runs, on any thread, when the compositor
    /// has finished with the buffer and the producer may draw into it again. On Apple
    /// platforms the host learns that from `IOSurfaceIsInUse` at the surface's next present,
    /// or when the surface goes, so a producer there never waits for a buffer to come back: it
    /// draws into one it knows is free, or a new one.
    fn present(&self, buffer: Buffer, released: Box<dyn FnOnce() + Send + Sync>);
}

/// What a producer draws into and hands over: the platform's own type.
#[cfg(target_vendor = "apple")]
pub type Buffer = objc2_core_foundation::CFRetained<objc2_io_surface::IOSurfaceRef>;

/// A platform with no host that composites: no buffer can exist, so
/// [`Surface::present`] cannot be called, and [`Platform::compositor`](crate::Platform::compositor)
/// answers `None`.
#[cfg(not(target_vendor = "apple"))]
pub enum Buffer {}

//! Native content the OS compositor shows beside the framework's picture: the capability a host
//! with a compositor hands out, and the composition layer a producer feeds from a thread of its
//! own. The Core Animation host's `os/apple/composition` module, in `inset-embedder-winit`, says
//! why it is shaped as it is. A producer that paces by the screen's refreshes waits on the
//! platform's [`VsyncWaiter`](crate::VsyncWaiter).
//!
//! A composition layer is a node of the compositor's tree, as Core Animation's `CALayer`,
//! DirectComposition's visual and Android's `SurfaceControl` are: it shows buffers presented to
//! it, or a layer the system draws itself, and holds children beneath it. It is made on its own
//! ([`Compositor::make_layer`]) and adopted by whoever places it, as `addSublayer` decides a
//! `CALayer`'s parent: a view's frame, whose [`Scene`](crate::Scene) places it through its
//! [`slices`](crate::slices), or another layer, whose [`CompositionLayer::present`] places its
//! children in its buffer's pixels, beneath its buffer. Each frame names every child it shows, as
//! a frame-based compositor takes them: Chromium's `SubmitCompositorFrame` lists the surfaces a
//! frame embeds, and Flutter's embedder hands its compositor each frame's layers. The host lands
//! a frame in the one transaction its compositor has for it (a `CATransaction`, a parent's
//! `wl_surface.commit`, `IDCompositionDevice::Commit`, a `SurfaceControl.Transaction`). A
//! program whose picture is itself a layer in another app's window shows its own layers as that
//! picture's children, and the placement of the picture moves, clips and scales the whole group
//! as one.
//!
//! The invariant: a layer is on screen only where its parent's last frame placed it, it has one
//! parent at a time, and the host, not the framework, owns the native object behind it.

use std::any::Any;
use std::sync::Arc;

use crate::Placement;

/// The host's compositor, as an app reaches it.
pub trait Compositor: Send + Sync {
    /// A layer placed nowhere yet, fed by a producer from any thread, and shown wherever the one
    /// that places it says: a view's scenes, or another layer's presents. `opaque` says whether
    /// its buffers carry alpha the compositor must blend; a layer shows its children through the
    /// clear pixels of its own buffer, so one with children is made not opaque.
    /// The layer goes when its last handle does: its producer's, and those of the frames that
    /// place it.
    fn make_layer(&self, opaque: bool) -> Arc<dyn CompositionLayer>;
}

/// One layer of the compositor's tree. Every method may be called from any thread.
///
/// The handle is the layer's identity: a frame places the `Arc` itself, so only a layer that
/// exists can be placed, and it lives while anything shows it. A host gets its own type back
/// from a placed layer by downcasting through `Any`.
pub trait CompositionLayer: Any + Send + Sync {
    /// Shows `buffer`, which fills the layer, and `children` beneath it, the bottom-most first,
    /// from the compositor's next frame on, in one step. Each child lies where its placement says
    /// in the buffer's pixels, as a scene places a layer in its view's, so the buffer's own
    /// placement moves, clips and scales it with it. A child the last present showed and this one
    /// does not name is let go; a layer another parent holds is not shown here, and is reported.
    ///
    /// Returns once the host holds the buffer, not once it is on screen. `released` runs, on any
    /// thread, when the compositor has finished with the buffer and the producer may draw into
    /// it again, as Wayland's `wl_buffer.release`. On Apple platforms the host learns that from
    /// `IOSurfaceIsInUse` at the layer's next present, or when the layer goes, so a producer
    /// there never waits for a buffer to come back: it draws into one it knows is free, or a new
    /// one. Core Animation takes no fence, so a producer that draws the buffer on the GPU
    /// presents it once that work is scheduled (`waitUntilScheduled`), as Chromium does before
    /// it hands Core Animation a surface.
    fn present(
        &self,
        buffer: HardwareBuffer,
        children: Vec<(Arc<dyn CompositionLayer>, Placement)>,
        released: Box<dyn FnOnce() + Send + Sync>,
    );
}

/// One block of pixels in memory the GPU, the system's compositor and other processes all reach,
/// which a producer draws into and presents: the platform's own type, Android's `HardwareBuffer`,
/// a dmabuf behind Wayland's `wl_buffer`, on Apple platforms an `IOSurface`.
#[cfg(target_vendor = "apple")]
pub type HardwareBuffer = objc2_core_foundation::CFRetained<objc2_io_surface::IOSurfaceRef>;

/// A platform with no host that composites: no buffer can exist, so
/// [`CompositionLayer::present`] cannot be called, and
/// [`Platform::compositor`](crate::Platform::compositor) answers `None`.
#[cfg(not(target_vendor = "apple"))]
pub enum HardwareBuffer {}

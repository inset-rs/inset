//! The buffers a producer presented to a composition layer, and when each goes back to it: the
//! one on screen until a present replaces it, then once the release fence the system hands over
//! for it signals (`releases`), at once for one never set on the layer's surface control, and
//! when the layer goes. A buffer presented with an acquire fence keeps it until a transaction
//! sets the buffer, which hands the fence to the system. A frame of a picture the system draws
//! onto the layer is one such buffer too, with how it is turned to fit the layer's box.

use std::os::fd::OwnedFd;

use crate::buffer_transform::BufferTransform;
use crate::hardware_buffers::AndroidHardwareBuffer;

use super::releases::Release;

/// The buffer presented last, the fence the compositor waits on before it reads it until a
/// transaction takes it, and whether a transaction has set it on the layer's surface control yet:
/// not while the layer has none, before its first placement.
pub(super) struct Shown {
    pub(super) buffer: AndroidHardwareBuffer,
    pub(super) acquire_fence: Option<OwnedFd>,
    pub(super) release: Release,
    pub(super) set: bool,
    /// How a system's frame is turned, shown whole within the layer's box; `None` for a
    /// producer's buffer, which fills the layer as it is.
    pub(super) fitted: Option<BufferTransform>,
}

/// The buffers presented to a layer whose release has not run: the one on screen, and the ones
/// replaced, which the compositor may still be reading.
pub(super) struct PresentedBuffers {
    on_screen: Option<Shown>,
    replaced: Vec<Release>,
}

impl PresentedBuffers {
    pub(super) fn new() -> PresentedBuffers {
        PresentedBuffers {
            on_screen: None,
            replaced: Vec::new(),
        }
    }

    /// `buffer` is on screen now, not yet set, to be read once `acquire_fence` signals, fitted to
    /// the layer's box where `fitted` says how it is turned; the one it replaces is answered, kept
    /// until its release runs.
    pub(super) fn present(
        &mut self,
        buffer: AndroidHardwareBuffer,
        acquire_fence: Option<OwnedFd>,
        fitted: Option<BufferTransform>,
        release: Release,
    ) -> Option<Shown> {
        self.replaced.retain(|replaced| !replaced.has_run());
        let previous = self.on_screen.replace(Shown {
            buffer,
            acquire_fence,
            release,
            set: false,
            fitted,
        });
        self.replaced
            .extend(previous.iter().map(|previous| previous.release.clone()));
        previous
    }

    /// Nothing is on screen from now on: the buffer that was is answered, kept until its release
    /// runs.
    pub(super) fn withdraw(&mut self) -> Option<Shown> {
        let previous = self.on_screen.take();
        self.replaced
            .extend(previous.iter().map(|previous| previous.release.clone()));
        previous
    }

    /// The buffer on screen; `None` before the first present, and once it is withdrawn.
    pub(super) fn on_screen(&self) -> Option<&Shown> {
        self.on_screen.as_ref()
    }

    /// The buffer on screen, to be set on the layer's surface control where it is not yet.
    pub(super) fn unset(&mut self) -> Option<&mut Shown> {
        self.on_screen.as_mut().filter(|shown| !shown.set)
    }

    /// Every release not yet run, for a layer that is going: its buffers are its producer's again.
    pub(super) fn release_all(&mut self) -> Vec<Release> {
        self.replaced
            .drain(..)
            .chain(self.on_screen.take().map(|shown| shown.release))
            .collect()
    }
}

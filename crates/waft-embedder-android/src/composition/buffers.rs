//! The buffers a producer presented to a composition layer, and when each goes back to it: the
//! one on screen until a present replaces it, then once the release fence the system hands over
//! for it signals (`releases`), at once for one never set on the layer's surface control, and
//! when the layer goes.

use crate::hardware_buffers::AndroidHardwareBuffer;

use super::releases::Release;

/// The buffer presented last, and whether a transaction has set it on the layer's surface
/// control yet: not while the layer has none, before its first placement.
pub(super) struct Shown {
    pub(super) buffer: AndroidHardwareBuffer,
    pub(super) release: Release,
    pub(super) set: bool,
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

    /// `buffer` is on screen now, not yet set; the one it replaces is answered, kept until its
    /// release runs.
    pub(super) fn present(
        &mut self,
        buffer: AndroidHardwareBuffer,
        release: Release,
    ) -> Option<Shown> {
        self.replaced.retain(|replaced| !replaced.has_run());
        let previous = self.on_screen.replace(Shown {
            buffer,
            release,
            set: false,
        });
        self.replaced
            .extend(previous.iter().map(|previous| previous.release.clone()));
        previous
    }

    /// The buffer on screen; `None` before the first present.
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

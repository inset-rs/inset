//! The buffers a producer presented to a surface, and when each goes back to it: once
//! `IOSurfaceIsInUse` answers false for a buffer that was replaced, or when the surface goes.

use inset_embedder::Buffer;
use objc2::runtime::AnyObject;
use objc2_io_surface::IOSurfaceRef;
use objc2_quartz_core::CALayer;

/// Calls each producer's release callback: its buffer is its own again.
pub(super) fn notify_released(callbacks: Vec<OnReleased>) {
    for callback in callbacks {
        callback();
    }
}

/// A producer's buffer, handed between threads.
pub(super) struct CrossThreadBuffer(pub(super) Buffer);

// SAFETY: an `IOSurface` is made to be shared between threads and processes: its retain count
// and use count are atomic. The binding cannot say so.
unsafe impl Send for CrossThreadBuffer {}

impl CrossThreadBuffer {
    pub(super) fn set_as_contents_of(&self, layer: &CALayer) {
        let surface: *const IOSurfaceRef = &*self.0;
        // SAFETY: a CF type is an Objective-C object, and a layer takes an IOSurface as its
        // contents; Apple documents only images there, and WebKit, Chromium and Flutter all
        // rely on it.
        unsafe { layer.setContents(Some(&*surface.cast::<AnyObject>())) };
    }
}

/// What a producer asked to run once the compositor has released its buffer.
pub(super) type OnReleased = Box<dyn FnOnce() + Send + Sync>;

/// The buffers presented to a surface and not yet released: the one on screen, and the ones it
/// replaced, which the compositor may still be reading.
pub(super) struct PresentedBuffers<B> {
    on_screen: Option<PresentedBuffer<B>>,
    replaced: Vec<PresentedBuffer<B>>,
}

/// A buffer presented to a surface, with what its producer asked to run once it is released.
struct PresentedBuffer<B> {
    buffer: B,
    on_released: OnReleased,
}

impl<B> PresentedBuffers<B> {
    pub(super) fn new() -> PresentedBuffers<B> {
        PresentedBuffers {
            on_screen: None,
            replaced: Vec::new(),
        }
    }

    /// `buffer` is on screen now, and the one that was is replaced.
    pub(super) fn present(&mut self, buffer: B, on_released: OnReleased) {
        let previous = self.on_screen.replace(PresentedBuffer {
            buffer,
            on_released,
        });
        self.replaced.extend(previous);
    }

    /// Releases the replaced buffers `in_use` no longer answers true for, and answers their
    /// callbacks for the caller to run.
    pub(super) fn release_unused(&mut self, in_use: impl Fn(&B) -> bool) -> Vec<OnReleased> {
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
    pub(super) fn release_all(&mut self) -> Vec<OnReleased> {
        self.replaced
            .drain(..)
            .chain(self.on_screen.take())
            .map(|presented| presented.on_released)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
}

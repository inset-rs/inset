//! Handing a buffer back to its producer once the compositor is done with it: a thread of its own
//! waits on the release fences the system hands over for replaced buffers, and runs each one's
//! release when its fence signals, at once for a buffer with no fence.
//!
//! The release fence is the moment, not the transaction's `OnComplete`: on the Pixel 2 XL the
//! callback arrives 5 to 6 ms before the fence of the buffer it replaced signals
//! (`experiments/2026-11-11-android-surface-control`, question 5).
//!
//! The invariant: a release runs once, whichever comes first, its fence or its layer going
//! (`Release::run`), and never on the thread that presents, which may hold the compositor's
//! transaction lock.

// Waiting on file descriptors is a C call; this module writes the `unsafe` for it.
#![allow(unsafe_code)]

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::{Arc, Mutex, PoisonError};

use waft_embedder::HardwareBuffer;

/// What a producer asked to run once the compositor has released its buffer.
pub(crate) type OnReleased = Box<dyn FnOnce() + Send + Sync>;

/// One buffer's release, run once by whoever reaches it first: its fence's waiter, or its layer
/// going. It holds the buffer until then.
#[derive(Clone)]
pub(crate) struct Release(Arc<Mutex<Option<Pending>>>);

struct Pending {
    _buffer: Arc<dyn HardwareBuffer>,
    on_released: OnReleased,
}

impl Release {
    pub(crate) fn new(buffer: Arc<dyn HardwareBuffer>, on_released: OnReleased) -> Release {
        Release(Arc::new(Mutex::new(Some(Pending {
            _buffer: buffer,
            on_released,
        }))))
    }

    /// Runs the producer's release, if nothing has yet.
    pub(crate) fn run(&self) {
        let pending = self.0.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(pending) = pending {
            (pending.on_released)();
        }
    }

    /// Whether it has run.
    pub(crate) fn has_run(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_none()
    }
}

/// The thread that waits on release fences, and the fences it waits on.
pub(crate) struct Releases {
    waiting: Arc<Waiting>,
}

/// The fences handed over and not yet signalled, and what wakes the thread to take new ones.
struct Waiting {
    handed: Mutex<Vec<(OwnedFd, Release)>>,
    wake: OwnedFd,
}

impl Releases {
    /// The waiting thread, started; `None` where the system gives no event to wake it by.
    pub(crate) fn start() -> Option<Releases> {
        // SAFETY: an eventfd of the process's own, closed when its `OwnedFd` drops.
        let wake = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if wake < 0 {
            return None;
        }
        let waiting = Arc::new(Waiting {
            handed: Mutex::new(Vec::new()),
            // SAFETY: the descriptor just made, owned from here on.
            wake: unsafe { OwnedFd::from_raw_fd(wake) },
        });
        let thread_waiting = Arc::clone(&waiting);
        std::thread::Builder::new()
            .name("waft-releases".into())
            .spawn(move || thread_waiting.serve())
            .ok()?;
        Some(Releases { waiting })
    }

    /// Runs `release` once `fence` signals, or at once where there is none.
    pub(crate) fn after(&self, fence: Option<OwnedFd>, release: Release) {
        let Some(fence) = fence else {
            release.run();
            return;
        };
        self.waiting.lock().push((fence, release));
        self.waiting.wake();
    }
}

impl Waiting {
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(OwnedFd, Release)>> {
        self.handed.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wakes the thread, to take the fences handed since it last looked.
    fn wake(&self) {
        let one: u64 = 1;
        // SAFETY: eight bytes to the process's own eventfd, which adds them to its count.
        unsafe { libc::write(self.wake.as_raw_fd(), (&raw const one).cast(), 8) };
    }

    /// Waits on every fence handed over, and on being woken for more, and runs the release of
    /// each fence that signals, for the life of the process.
    fn serve(&self) {
        loop {
            let mut polled = self.polled();
            // SAFETY: `polled` holds `polled.len()` entries, of descriptors open for the call:
            // the eventfd and the fences, which only this thread takes out of `handed`.
            let ready = unsafe { libc::poll(polled.as_mut_ptr(), polled.len() as _, -1) };
            if ready <= 0 {
                continue;
            }
            self.drain_wake();
            let signalled: Vec<i32> = polled[1..]
                .iter()
                .filter(|entry| entry.revents != 0)
                .map(|entry| entry.fd)
                .collect();
            self.release_signalled(&signalled);
        }
    }

    /// The eventfd, then every fence, as `poll` takes them.
    fn polled(&self) -> Vec<libc::pollfd> {
        let entry = |fd: i32| libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        std::iter::once(entry(self.wake.as_raw_fd()))
            .chain(
                self.lock()
                    .iter()
                    .map(|(fence, _)| entry(fence.as_raw_fd())),
            )
            .collect()
    }

    fn drain_wake(&self) {
        let mut count: u64 = 0;
        // SAFETY: eight bytes from the process's own eventfd, which never blocks.
        unsafe { libc::read(self.wake.as_raw_fd(), (&raw mut count).cast(), 8) };
    }

    /// Runs the release of each fence among `signalled`, outside the lock, and closes the fence.
    fn release_signalled(&self, signalled: &[i32]) {
        let released: Vec<(OwnedFd, Release)> = {
            let mut handed = self.lock();
            let (done, waiting) = std::mem::take(&mut *handed)
                .into_iter()
                .partition(|(fence, _)| signalled.contains(&fence.as_raw_fd()));
            *handed = waiting;
            done
        };
        for (_fence, release) in released {
            release.run();
        }
    }
}

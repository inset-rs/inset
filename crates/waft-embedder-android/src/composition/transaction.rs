//! One frame of the compositor's tree, as layers build it: the `ASurfaceTransaction`, the buffers
//! it replaces, which go back to their producers once their release fences signal, and what must
//! wait until it is applied, outside the compositor's lock.
//!
//! The invariant: transactions are built and applied one at a time, under the compositor's lock,
//! so the buffers a layer records as replaced are the ones the system reports the release of, in
//! the order it applies them; and nothing a transaction lets go of, a layer or a producer's
//! release, runs inside that lock.

use std::sync::{Arc, Mutex, PoisonError};

use crate::surface_control::{PreviousRelease, SurfaceControl, Transaction};

use super::SurfaceControlLayer;
use super::releases::{Release, Releases};

/// What every layer shares: the lock transactions are built and applied under, and the thread that
/// waits on release fences.
pub(super) struct Transactions {
    lock: Mutex<()>,
    releases: Arc<Releases>,
}

impl Transactions {
    pub(super) fn new(releases: Releases) -> Transactions {
        Transactions {
            lock: Mutex::new(()),
            releases: Arc::new(releases),
        }
    }

    /// Builds one transaction with `build` and applies it, under the lock; what it let go of goes
    /// after.
    pub(super) fn transact(&self, build: impl FnOnce(&mut LayerTransaction)) {
        let held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut transaction = LayerTransaction {
            transaction: Transaction::new(),
            replaced: Vec::new(),
            afterwards: Afterwards::default(),
        };
        build(&mut transaction);
        let afterwards = transaction.apply(&self.releases);
        drop(held);
        afterwards.finish();
    }
}

/// A transaction being built, and what follows from it.
pub(super) struct LayerTransaction {
    pub(super) transaction: Transaction,
    /// The buffers it replaces, each on the surface control it replaces it on.
    replaced: Vec<(Arc<SurfaceControl>, Release)>,
    afterwards: Afterwards,
}

/// What waits until a transaction is applied and the lock let go.
#[derive(Default)]
struct Afterwards {
    /// Releases of buffers never shown, which run at once.
    released: Vec<Release>,
    /// Layers let go, held until the transaction that takes them down is applied.
    kept: Vec<Arc<SurfaceControlLayer>>,
}

impl LayerTransaction {
    /// `release`'s buffer, shown on `surface` until this transaction, is replaced: it goes back to
    /// its producer once the system's release fence for it signals.
    pub(super) fn replaces(&mut self, surface: &Arc<SurfaceControl>, release: Release) {
        self.replaced.push((Arc::clone(surface), release));
    }

    /// `release`'s buffer was never shown: it goes back to its producer once this is applied.
    pub(super) fn release_unshown(&mut self, release: Release) {
        self.afterwards.released.push(release);
    }

    /// Keeps `layers` until this is applied, outside the lock.
    pub(super) fn keep(&mut self, layers: impl IntoIterator<Item = Arc<SurfaceControlLayer>>) {
        self.afterwards.kept.extend(layers);
    }

    /// Applies the transaction, the release of each buffer it replaces handed to `releases` once
    /// its frame is on screen, and answers what waits for the lock to go.
    fn apply(mut self, releases: &Arc<Releases>) -> Afterwards {
        if !self.replaced.is_empty() {
            let replaced = std::mem::take(&mut self.replaced);
            let releases = Arc::clone(releases);
            self.transaction.on_complete(move |completion| {
                for (surface, release) in replaced {
                    match completion.previous_release(&surface) {
                        PreviousRelease::Fence(fence) => releases.after(Some(fence), release),
                        PreviousRelease::Free | PreviousRelease::NotReported => release.run(),
                    }
                }
            });
        }
        self.transaction.apply();
        self.afterwards
    }
}

impl Afterwards {
    fn finish(self) {
        for release in self.released {
            release.run();
        }
        drop(self.kept);
    }
}

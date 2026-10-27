//! DeferralManager.h: how an event lets its handler finish asynchronous work before the control
//! goes on. The control prepares a generation, raises the event, and continues once every
//! deferral the handlers took from the event's arguments is complete.

use inset_foundation::App;
use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

/// `ERROR_DEFERRAL_COMPLETED`: a deferral was asked for, or completed, after its generation had
/// finished, or completed twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeferralCompleted;

impl fmt::Display for DeferralCompleted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the deferral has already completed")
    }
}

impl std::error::Error for DeferralCompleted {}

/// What a generation runs once its deferrals are all complete.
type Continuation = Box<dyn FnOnce(&mut App)>;

/// `DeferralManager`: counts the outstanding deferrals of one generation, and runs the
/// generation's continuation once they are all complete. A deferral keeps its manager alive, so
/// one completed after its control has moved on finds no continuation and does nothing.
pub(crate) struct DeferralManager {
    count: Cell<u32>,
    generation: Cell<u64>,
    callback: RefCell<Option<Continuation>>,
}

impl DeferralManager {
    pub(crate) fn new() -> Rc<DeferralManager> {
        Rc::new(DeferralManager {
            count: Cell::new(0),
            generation: Cell::new(1),
            callback: RefCell::new(None),
        })
    }

    /// `Prepare`: begins a generation and returns its id, or `None` while one is still in
    /// progress. The generation stays open until [`continue_with`](Self::continue_with).
    pub(crate) fn prepare(&self) -> Option<u64> {
        if self.count.get() != 0 {
            return None;
        }
        debug_assert!(self.callback.borrow().is_none());
        self.count.set(1);
        Some(self.generation.get())
    }

    /// `ContinueWith`: runs `callback` once the generation's deferrals are all complete, at once
    /// when none is outstanding.
    pub(crate) fn continue_with(&self, app: &mut App, callback: impl FnOnce(&mut App) + 'static) {
        *self.callback.borrow_mut() = Some(Box::new(callback));
        self.decrement(app);
    }

    /// `Disconnect`: drops the continuation, for a control whose state has moved on.
    pub(crate) fn disconnect(&self) {
        self.callback.borrow_mut().take();
    }

    /// `GetDeferral`: a deferral of `generation`, which delays the continuation until it is
    /// complete.
    pub(crate) fn get_deferral(
        self: &Rc<Self>,
        generation: u64,
    ) -> Result<Deferral, DeferralCompleted> {
        if self.count.get() == 0 || generation != self.generation.get() {
            return Err(DeferralCompleted);
        }
        self.count.set(self.count.get() + 1);
        Ok(Deferral {
            manager: RefCell::new(Some(Rc::clone(self))),
            generation,
        })
    }

    /// `CompleteDeferral`.
    fn complete_deferral(&self, app: &mut App, generation: u64) -> Result<(), DeferralCompleted> {
        if self.count.get() == 0 || generation != self.generation.get() {
            return Err(DeferralCompleted);
        }
        self.decrement(app);
        Ok(())
    }

    /// `DecrementDeferralCount`: the last outstanding deferral ends the generation and runs its
    /// continuation.
    fn decrement(&self, app: &mut App) {
        self.count.set(self.count.get() - 1);
        if self.count.get() != 0 {
            return;
        }
        let callback = self.callback.borrow_mut().take();
        self.generation.set(self.generation.get() + 1);
        if let Some(callback) = callback {
            callback(app);
        }
    }
}

/// A deferral an event's handler took: the control waits for [`complete`](Self::complete)
/// before it goes on.
pub struct Deferral {
    manager: RefCell<Option<Rc<DeferralManager>>>,
    generation: u64,
}

impl Deferral {
    /// `Complete`: lets the control go on, once its other deferrals are complete too.
    pub fn complete(&self, app: &mut App) -> Result<(), DeferralCompleted> {
        let manager = self.manager.borrow_mut().take().ok_or(DeferralCompleted)?;
        manager.complete_deferral(app, self.generation)
    }
}

impl fmt::Debug for Deferral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Deferral")
            .field("complete", &self.manager.borrow().is_none())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_foundation::AppCell;

    #[test]
    fn the_continuation_waits_for_every_deferral_of_its_generation() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let manager = DeferralManager::new();
        let ran = Rc::new(Cell::new(false));
        let generation = manager.prepare().unwrap();
        let first = manager.get_deferral(generation).unwrap();
        let second = manager.get_deferral(generation).unwrap();
        assert_eq!(manager.prepare(), None, "a generation is still in progress");
        let continued = ran.clone();
        manager.continue_with(&mut app, move |_| continued.set(true));
        first.complete(&mut app).unwrap();
        assert!(!ran.get());
        second.complete(&mut app).unwrap();
        assert!(ran.get());
        assert_eq!(second.complete(&mut app), Err(DeferralCompleted));
        assert!(manager.get_deferral(generation).is_err());
    }

    #[test]
    fn without_deferrals_the_continuation_runs_at_once_and_disconnecting_drops_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let manager = DeferralManager::new();
        let runs = Rc::new(Cell::new(0));
        let generation = manager.prepare().unwrap();
        let continued = runs.clone();
        manager.continue_with(&mut app, move |_| continued.set(continued.get() + 1));
        assert_eq!(runs.get(), 1);
        assert!(generation > 0);
        let generation = manager.prepare().unwrap();
        let deferral = manager.get_deferral(generation).unwrap();
        let continued = runs.clone();
        manager.continue_with(&mut app, move |_| continued.set(continued.get() + 1));
        manager.disconnect();
        deferral.complete(&mut app).unwrap();
        assert_eq!(runs.get(), 1);
    }
}

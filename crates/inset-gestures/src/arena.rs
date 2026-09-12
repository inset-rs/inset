//! Flutter counterpart: `gestures/arena.dart`.

use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_foundation::{App, Handle, HandleId, Listener};

use crate::debug::debug_print_gesture_arena_diagnostics;

/// Whether the gesture was accepted or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureDisposition {
    /// This gesture was accepted as the interpretation of the user's input.
    Accepted,
    /// This gesture was rejected as the interpretation of the user's input.
    Rejected,
}

/// Represents an object participating in an arena.
///
/// Receives callbacks from the GestureArena to notify the object when it wins
/// or loses a gesture negotiation. Exactly one of [`accept_gesture`](Self::accept_gesture) or
/// [`reject_gesture`](Self::reject_gesture) will be called for each arena this member was added to,
/// regardless of what caused the arena to be resolved. For example, if a
/// member resolves the arena itself, that member still receives an
/// [`accept_gesture`](Self::accept_gesture) callback.
pub trait GestureArenaMember: 'static {
    /// Called when this member wins the arena for the given pointer id.
    fn accept_gesture(&self, app: &mut App, pointer: i64);

    /// Called when this member loses the arena for the given pointer id.
    fn reject_gesture(&self, app: &mut App, pointer: i64);

    /// Dart object identity for this member.
    fn member_id(&self) -> HandleId;
}

#[derive(Clone)]
pub(crate) struct MemberBox(Rc<dyn GestureArenaMember>);

impl MemberBox {
    pub(crate) fn new(member: impl GestureArenaMember) -> MemberBox {
        MemberBox(Rc::new(member))
    }

    pub(crate) fn member_id(&self) -> HandleId {
        self.0.member_id()
    }

    pub(crate) fn accept_gesture(&self, app: &mut App, pointer: i64) {
        self.0.accept_gesture(app, pointer);
    }

    pub(crate) fn reject_gesture(&self, app: &mut App, pointer: i64) {
        self.0.reject_gesture(app, pointer);
    }
}

impl PartialEq for MemberBox {
    fn eq(&self, other: &MemberBox) -> bool {
        self.member_id() == other.member_id()
    }
}

/// An interface to pass information to an arena.
///
/// A given [`GestureArenaMember`] can have multiple entries in multiple arenas
/// with different pointer ids.
///
/// Dart's `_CombiningGestureArenaEntry implements GestureArenaEntry`. Both
/// resolve through this type.
type ResolveEntry = dyn Fn(&mut App, GestureDisposition);

#[derive(Clone)]
pub struct GestureArenaEntry {
    resolve: Rc<ResolveEntry>,
}

impl GestureArenaEntry {
    pub(crate) fn from_resolve(
        resolve: impl Fn(&mut App, GestureDisposition) + 'static,
    ) -> GestureArenaEntry {
        GestureArenaEntry {
            resolve: Rc::new(resolve),
        }
    }

    /// Call this member to claim victory (with accepted) or admit defeat (with rejected).
    ///
    /// It's fine to attempt to resolve a gesture recognizer for an arena that is
    /// already resolved.
    pub fn resolve(&self, app: &mut App, disposition: GestureDisposition) {
        (self.resolve)(app, disposition);
    }
}

struct GestureArena {
    /// Distinguishes this opening from a later arena on the same pointer.
    /// Dart compares `_GestureArena` instances.
    id: u64,
    members: Vec<MemberBox>,
    is_open: bool,
    is_held: bool,
    has_pending_sweep: bool,
    /// If a member attempts to win while the arena is still open, it becomes the
    /// "eager winner". We look for an eager winner when closing the arena to new
    /// participants, and if there is one, we resolve the arena in its favor at
    /// that time.
    eager_winner: Option<MemberBox>,
}

impl GestureArena {
    fn new(id: u64) -> GestureArena {
        GestureArena {
            id,
            members: Vec::new(),
            is_open: true,
            is_held: false,
            has_pending_sweep: false,
            eager_winner: None,
        }
    }

    fn add(&mut self, member: MemberBox) {
        debug_assert!(self.is_open);
        self.members.push(member);
    }
}

impl Debug for GestureArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.members.is_empty() {
            write!(f, "<empty>")?;
        } else {
            for (i, member) in self.members.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{:?}", member.member_id())?;
                if self.eager_winner.as_ref() == Some(member) {
                    write!(f, " (eager winner)")?;
                }
            }
        }
        if self.is_open {
            write!(f, " [open]")?;
        }
        if self.is_held {
            write!(f, " [held]")?;
        }
        if self.has_pending_sweep {
            write!(f, " [hasPendingSweep]")?;
        }
        Ok(())
    }
}

/// Used for disambiguating the meaning of sequences of pointer events.
///
/// The first member to accept or the last member to not reject wins.
///
/// See <https://flutter.dev/to/gesture-disambiguation> for more
/// information about the role this class plays in the gesture system.
///
/// To debug problems with gestures, consider using
/// [`debug_print_gesture_arena_diagnostics`].
pub struct GestureArenaManager {
    next_id: u64,
    arenas: HashMap<i64, GestureArena>,
}

impl GestureArenaManager {
    /// Creates an empty manager.
    pub fn new(app: &mut App) -> Handle<GestureArenaManager> {
        app.create(GestureArenaManager {
            next_id: 0,
            arenas: HashMap::new(),
        })
    }

    /// Adds a new member (e.g., gesture recognizer) to the arena.
    pub fn add(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        member: impl GestureArenaMember,
    ) -> GestureArenaEntry {
        let member = MemberBox::new(member);
        {
            let data = app.get_mut(self);
            if !data.arenas.contains_key(&pointer) {
                debug_log_diagnostic(pointer, "★ Opening new gesture arena.", None);
                let id = data.next_id;
                data.next_id += 1;
                data.arenas.insert(pointer, GestureArena::new(id));
            }
            data.arenas.get_mut(&pointer).unwrap().add(member.clone());
        }
        debug_log_diagnostic(pointer, &format!("Adding: {:?}", member.member_id()), None);
        GestureArenaEntry::from_resolve(move |app, disposition| {
            self.resolve(app, pointer, &member, disposition);
        })
    }

    /// Prevents new members from entering the arena.
    ///
    /// Called after the framework has finished dispatching the pointer down event.
    pub fn close(self: Handle<Self>, app: &mut App, pointer: i64) {
        let arena_id = {
            let Some(state) = app.get_mut(self).arenas.get_mut(&pointer) else {
                return; // This arena either never existed or has been resolved.
            };
            state.is_open = false;
            state.id
        };
        let count = app
            .get(self)
            .arenas
            .get(&pointer)
            .map(|state| state.members.len());
        debug_log_diagnostic(pointer, "Closing", count);
        self.try_to_resolve_arena(app, pointer, arena_id);
    }

    /// Forces resolution of the arena, giving the win to the first member.
    ///
    /// Sweep is typically after all the other processing for a `PointerUpEvent`
    /// have taken place. It ensures that multiple passive gestures do not cause a
    /// stalemate that prevents the user from interacting with the app.
    ///
    /// Recognizers that wish to delay resolving an arena past `PointerUpEvent`
    /// should call [`hold`](Self::hold) to delay sweep until [`release`](Self::release) is called.
    ///
    /// See also:
    ///
    ///  * [`hold`](Self::hold)
    ///  * [`release`](Self::release)
    pub fn sweep(self: Handle<Self>, app: &mut App, pointer: i64) {
        let (is_held, member_count) = {
            let Some(state) = app.get(self).arenas.get(&pointer) else {
                return; // This arena either never existed or has been resolved.
            };
            debug_assert!(!state.is_open);
            (state.is_held, state.members.len())
        };
        if is_held {
            app.get_mut(self)
                .arenas
                .get_mut(&pointer)
                .unwrap()
                .has_pending_sweep = true;
            debug_log_diagnostic(pointer, "Delaying sweep", Some(member_count));
            return; // This arena is being held for a long-lived member.
        }
        debug_log_diagnostic(pointer, "Sweeping", Some(member_count));
        let Some(state) = app.get_mut(self).arenas.remove(&pointer) else {
            return;
        };
        if let Some(first) = state.members.first() {
            debug_log_diagnostic(pointer, &format!("Winner: {:?}", first.member_id()), None);
            first.accept_gesture(app, pointer);
            for member in state.members.iter().skip(1) {
                member.reject_gesture(app, pointer);
            }
        }
    }

    /// Prevents the arena from being swept.
    ///
    /// Typically, a winner is chosen in an arena after all the other
    /// `PointerUpEvent` processing by [`sweep`](Self::sweep). If a recognizer wishes to delay
    /// resolving an arena past `PointerUpEvent`, the recognizer can [`hold`](Self::hold) the
    /// arena open using this function. To release such a hold and let the arena
    /// resolve, call [`release`](Self::release).
    ///
    /// See also:
    ///
    ///  * [`sweep`](Self::sweep)
    ///  * [`release`](Self::release)
    pub fn hold(self: Handle<Self>, app: &mut App, pointer: i64) {
        let Some(state) = app.get_mut(self).arenas.get_mut(&pointer) else {
            return; // This arena either never existed or has been resolved.
        };
        state.is_held = true;
        let count = state.members.len();
        debug_log_diagnostic(pointer, "Holding", Some(count));
    }

    /// Releases a hold, allowing the arena to be swept.
    ///
    /// If a sweep was attempted on a held arena, the sweep will be done
    /// on release.
    ///
    /// See also:
    ///
    ///  * [`sweep`](Self::sweep)
    ///  * [`hold`](Self::hold)
    pub fn release(self: Handle<Self>, app: &mut App, pointer: i64) {
        let has_pending_sweep = {
            let Some(state) = app.get_mut(self).arenas.get_mut(&pointer) else {
                return; // This arena either never existed or has been resolved.
            };
            state.is_held = false;
            let count = state.members.len();
            debug_log_diagnostic(pointer, "Releasing", Some(count));
            state.has_pending_sweep
        };
        if has_pending_sweep {
            self.sweep(app, pointer);
        }
    }

    fn resolve(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        member: &MemberBox,
        disposition: GestureDisposition,
    ) {
        let Some(arena_id) = app.get(self).arenas.get(&pointer).map(|state| {
            debug_assert!(state.members.contains(member));
            state.id
        }) else {
            return; // This arena has already resolved.
        };
        match disposition {
            GestureDisposition::Accepted => {
                debug_log_diagnostic(
                    pointer,
                    &format!("Accepting: {:?}", member.member_id()),
                    None,
                );
                let is_open = app
                    .get(self)
                    .arenas
                    .get(&pointer)
                    .is_some_and(|state| state.id == arena_id && state.is_open);
                if is_open {
                    let state = app.get_mut(self).arenas.get_mut(&pointer).unwrap();
                    if state.eager_winner.is_none() {
                        state.eager_winner = Some(member.clone());
                    }
                } else {
                    debug_log_diagnostic(
                        pointer,
                        &format!("Self-declared winner: {:?}", member.member_id()),
                        None,
                    );
                    self.resolve_in_favor_of(app, pointer, arena_id, member);
                }
            }
            GestureDisposition::Rejected => {
                debug_log_diagnostic(
                    pointer,
                    &format!("Rejecting: {:?}", member.member_id()),
                    None,
                );
                let should_try = {
                    let Some(state) = app.get_mut(self).arenas.get_mut(&pointer) else {
                        return;
                    };
                    if state.id != arena_id {
                        return;
                    }
                    if state.eager_winner.as_ref() == Some(member) {
                        state.eager_winner = None;
                    }
                    state.members.retain(|m| m != member);
                    !state.is_open
                };
                member.reject_gesture(app, pointer);
                if should_try {
                    self.try_to_resolve_arena(app, pointer, arena_id);
                }
            }
        }
    }

    fn try_to_resolve_arena(self: Handle<Self>, app: &mut App, pointer: i64, arena_id: u64) {
        let Some((member_count, eager_winner)) =
            app.get(self).arenas.get(&pointer).and_then(|state| {
                if state.id != arena_id {
                    return None;
                }
                debug_assert!(!state.is_open);
                Some((state.members.len(), state.eager_winner.clone()))
            })
        else {
            return;
        };
        if member_count == 1 {
            app.schedule_microtask(Listener::new(move |app| {
                self.resolve_by_default(app, pointer, arena_id);
            }));
        } else if member_count == 0 {
            app.get_mut(self).arenas.remove(&pointer);
            debug_log_diagnostic(pointer, "Arena empty.", None);
        } else if let Some(winner) = eager_winner {
            debug_log_diagnostic(
                pointer,
                &format!("Eager winner: {:?}", winner.member_id()),
                None,
            );
            self.resolve_in_favor_of(app, pointer, arena_id, &winner);
        }
    }

    fn resolve_by_default(self: Handle<Self>, app: &mut App, pointer: i64, arena_id: u64) {
        let Some(winner) = app.get(self).arenas.get(&pointer).and_then(|state| {
            if state.id != arena_id {
                return None;
            }
            debug_assert!(!state.is_open);
            debug_assert_eq!(state.members.len(), 1);
            state.members.first().cloned()
        }) else {
            return; // This arena has already resolved.
        };
        app.get_mut(self).arenas.remove(&pointer);
        debug_log_diagnostic(
            pointer,
            &format!("Default winner: {:?}", winner.member_id()),
            None,
        );
        winner.accept_gesture(app, pointer);
    }

    fn resolve_in_favor_of(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        arena_id: u64,
        member: &MemberBox,
    ) {
        let Some(members) = app.get(self).arenas.get(&pointer).and_then(|state| {
            if state.id != arena_id {
                return None;
            }
            debug_assert!(
                state.eager_winner.is_none() || state.eager_winner.as_ref() == Some(member)
            );
            debug_assert!(!state.is_open);
            Some(state.members.clone())
        }) else {
            return;
        };
        app.get_mut(self).arenas.remove(&pointer);
        for rejected in &members {
            if rejected != member {
                rejected.reject_gesture(app, pointer);
            }
        }
        member.accept_gesture(app, pointer);
    }
}

fn debug_log_diagnostic(pointer: i64, message: &str, member_count: Option<usize>) {
    if cfg!(debug_assertions) && debug_print_gesture_arena_diagnostics() {
        let suffix = match member_count {
            Some(count) => {
                let s = if count != 1 { "s" } else { "" };
                format!(" with {count} member{s}.")
            }
            None => String::new(),
        };
        eprintln!(
            "Gesture arena {:<4} ❙ {message}{suffix}",
            pointer.to_string()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_foundation::{App, AppCell, Handle};

    const PRIMARY_KEY: i64 = 4;

    struct TestMember {
        accept_ran: bool,
        reject_ran: bool,
    }

    impl TestMember {
        fn new(app: &mut App) -> Handle<TestMember> {
            app.create(TestMember {
                accept_ran: false,
                reject_ran: false,
            })
        }

        fn accept_ran(self: Handle<Self>, app: &App) -> bool {
            app.get(self).accept_ran
        }

        fn reject_ran(self: Handle<Self>, app: &App) -> bool {
            app.get(self).reject_ran
        }
    }

    impl GestureArenaMember for Handle<TestMember> {
        fn accept_gesture(&self, app: &mut App, pointer: i64) {
            assert_eq!(pointer, PRIMARY_KEY);
            app.get_mut(*self).accept_ran = true;
        }

        fn reject_gesture(&self, app: &mut App, pointer: i64) {
            assert_eq!(pointer, PRIMARY_KEY);
            app.get_mut(*self).reject_ran = true;
        }

        fn member_id(&self) -> HandleId {
            self.id()
        }
    }

    struct GestureTester {
        cell: Rc<AppCell>,
        arena: Handle<GestureArenaManager>,
        first: Handle<TestMember>,
        second: Handle<TestMember>,
        first_entry: Option<GestureArenaEntry>,
        second_entry: Option<GestureArenaEntry>,
    }

    impl GestureTester {
        fn new() -> GestureTester {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let first = TestMember::new(&mut app);
            let second = TestMember::new(&mut app);
            let arena = GestureArenaManager::new(&mut app);
            drop(app);
            GestureTester {
                cell,
                arena,
                first,
                second,
                first_entry: None,
                second_entry: None,
            }
        }

        fn add_first(&mut self) {
            self.first_entry = Some(self.arena.add(
                &mut self.cell.borrow_mut(),
                PRIMARY_KEY,
                self.first,
            ));
        }

        fn add_second(&mut self) {
            self.second_entry = Some(self.arena.add(
                &mut self.cell.borrow_mut(),
                PRIMARY_KEY,
                self.second,
            ));
        }

        fn expect_nothing(&self) {
            assert!(!self.first.accept_ran(&self.cell.borrow()));
            assert!(!self.first.reject_ran(&self.cell.borrow()));
            assert!(!self.second.accept_ran(&self.cell.borrow()));
            assert!(!self.second.reject_ran(&self.cell.borrow()));
        }

        fn expect_first_win(&self) {
            assert!(self.first.accept_ran(&self.cell.borrow()));
            assert!(!self.first.reject_ran(&self.cell.borrow()));
            assert!(!self.second.accept_ran(&self.cell.borrow()));
            assert!(self.second.reject_ran(&self.cell.borrow()));
        }

        fn expect_second_win(&self) {
            assert!(!self.first.accept_ran(&self.cell.borrow()));
            assert!(self.first.reject_ran(&self.cell.borrow()));
            assert!(self.second.accept_ran(&self.cell.borrow()));
            assert!(!self.second.reject_ran(&self.cell.borrow()));
        }
    }

    #[test]
    fn should_win_by_accepting() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .first_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester.expect_first_win();
    }

    #[test]
    fn should_win_by_sweep() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .arena
            .sweep(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_first_win();
    }

    #[test]
    fn should_win_on_release_after_hold_sweep_release() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .arena
            .hold(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .arena
            .sweep(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .arena
            .release(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_first_win();
    }

    #[test]
    fn should_win_on_sweep_after_hold_release_sweep() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .arena
            .hold(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .arena
            .release(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .arena
            .sweep(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_first_win();
    }

    #[test]
    fn only_first_winner_should_win() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .first_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester
            .second_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester.expect_first_win();
    }

    #[test]
    fn only_first_winner_should_win_regardless_of_order() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_nothing();
        tester
            .second_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester
            .first_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester.expect_second_win();
    }

    #[test]
    fn win_before_close_is_delayed_to_close() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester.expect_nothing();
        tester
            .first_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester.expect_nothing();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_first_win();
    }

    #[test]
    fn win_before_close_is_delayed_and_only_first_eager_winner() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester.expect_nothing();
        tester
            .first_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester
            .second_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester.expect_nothing();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_first_win();
    }

    #[test]
    fn win_before_close_delayed_eager_winner_regardless_of_order() {
        let mut tester = GestureTester::new();
        tester.add_first();
        tester.add_second();
        tester.expect_nothing();
        tester
            .second_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester
            .first_entry
            .as_ref()
            .unwrap()
            .resolve(&mut tester.cell.borrow_mut(), GestureDisposition::Accepted);
        tester.expect_nothing();
        tester
            .arena
            .close(&mut tester.cell.borrow_mut(), PRIMARY_KEY);
        tester.expect_second_win();
    }

    #[test]
    fn eager_winner_cleared_when_it_rejects_while_open() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let arena = GestureArenaManager::new(&mut app);
        let member_a = TestMember::new(&mut app);
        let member_b = TestMember::new(&mut app);
        let member_c = TestMember::new(&mut app);

        let entry_a = arena.add(&mut app, PRIMARY_KEY, member_a);
        arena.add(&mut app, PRIMARY_KEY, member_b);
        arena.add(&mut app, PRIMARY_KEY, member_c);

        entry_a.resolve(&mut app, GestureDisposition::Accepted);
        assert!(!member_a.accept_ran(&app));
        assert!(!member_a.reject_ran(&app));

        entry_a.resolve(&mut app, GestureDisposition::Rejected);
        assert!(member_a.reject_ran(&app));
        assert!(!member_a.accept_ran(&app));

        arena.close(&mut app, PRIMARY_KEY);
        assert!(!member_a.accept_ran(&app));
        assert!(!member_b.reject_ran(&app));
        assert!(!member_c.reject_ran(&app));

        arena.sweep(&mut app, PRIMARY_KEY);
        assert!(member_b.accept_ran(&app));
        assert!(!member_b.reject_ran(&app));
        assert!(member_c.reject_ran(&app));
        assert!(!member_c.accept_ran(&app));
    }

    #[test]
    fn sole_member_wins_on_close_after_microtask() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let arena = GestureArenaManager::new(&mut app);
        let member = TestMember::new(&mut app);
        arena.add(&mut app, PRIMARY_KEY, member);
        arena.close(&mut app, PRIMARY_KEY);
        assert!(!member.accept_ran(&app));
        app.drain_microtasks();
        assert!(member.accept_ran(&app));
    }
}

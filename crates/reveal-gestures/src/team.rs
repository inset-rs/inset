//! Flutter counterpart: `gestures/team.dart`.

use std::collections::HashMap;

use reveal_foundation::{App, Handle, HandleId};

use crate::arena::{GestureArenaEntry, GestureArenaMember, GestureDisposition, MemberBox};
use crate::binding::GestureBinding;

struct CombiningData {
    owner: GestureArenaTeam,
    pointer: i64,
    members: Vec<MemberBox>,
    resolved: bool,
    winner: Option<MemberBox>,
    entry: Option<GestureArenaEntry>,
}

#[derive(Clone, Copy)]
struct CombiningGestureArenaMember(Handle<CombiningData>);

impl CombiningGestureArenaMember {
    fn add_member(self, app: &mut App, pointer: i64, member: MemberBox) -> GestureArenaEntry {
        debug_assert!(!app.get(self.0).resolved);
        debug_assert_eq!(app.get(self.0).pointer, pointer);
        app.get_mut(self.0).members.push(member.clone());
        if app.get(self.0).entry.is_none() {
            let entry = GestureBinding::instance(app)
                .gesture_arena(app)
                .add(app, pointer, self);
            app.get_mut(self.0).entry = Some(entry);
        }
        GestureArenaEntry::from_resolve(move |app, disposition| {
            self.resolve_member(app, &member, disposition);
        })
    }

    fn close(self, app: &mut App) {
        debug_assert!(!app.get(self.0).resolved);
        app.get_mut(self.0).resolved = true;
        let owner = app.get(self.0).owner;
        let pointer = app.get(self.0).pointer;
        let removed = app.get_mut(owner.0).combiners.remove(&pointer);
        debug_assert!(removed.is_some_and(|combiner| combiner.0 == self.0));
    }

    fn resolve_member(self, app: &mut App, member: &MemberBox, disposition: GestureDisposition) {
        if app.get(self.0).resolved {
            return;
        }
        match disposition {
            GestureDisposition::Accepted => {
                let captain = {
                    let owner = app.get(self.0).owner;
                    app.get(owner.0).captain.clone()
                };
                {
                    let data = app.get_mut(self.0);
                    if data.winner.is_none() {
                        data.winner = captain.or_else(|| Some(member.clone()));
                    }
                }
                let entry = app.get(self.0).entry.clone().unwrap();
                entry.resolve(app, disposition);
            }
            GestureDisposition::Rejected => {
                let pointer = app.get(self.0).pointer;
                let empty = {
                    let data = app.get_mut(self.0);
                    data.members.retain(|m| m != member);
                    data.members.is_empty()
                };
                member.reject_gesture(app, pointer);
                if empty {
                    let entry = app.get(self.0).entry.clone().unwrap();
                    entry.resolve(app, disposition);
                }
            }
        }
    }
}

impl GestureArenaMember for CombiningGestureArenaMember {
    fn accept_gesture(&self, app: &mut App, pointer: i64) {
        debug_assert_eq!(app.get(self.0).pointer, pointer);
        debug_assert!(app.get(self.0).winner.is_some() || !app.get(self.0).members.is_empty());
        self.close(app);
        let captain = {
            let owner = app.get(self.0).owner;
            app.get(owner.0).captain.clone()
        };
        let (winner, members) = {
            let data = app.get_mut(self.0);
            if data.winner.is_none() {
                data.winner = captain.or_else(|| data.members.first().cloned());
            }
            (
                data.winner.clone().expect("combiner has a winner"),
                data.members.clone(),
            )
        };
        for member in &members {
            if member != &winner {
                member.reject_gesture(app, pointer);
            }
        }
        winner.accept_gesture(app, pointer);
    }

    fn reject_gesture(&self, app: &mut App, pointer: i64) {
        debug_assert_eq!(app.get(self.0).pointer, pointer);
        self.close(app);
        let members = app.get(self.0).members.clone();
        for member in &members {
            member.reject_gesture(app, pointer);
        }
    }

    fn member_id(&self) -> HandleId {
        self.0.id()
    }
}

pub(crate) struct GestureArenaTeamData {
    combiners: HashMap<i64, CombiningGestureArenaMember>,
    captain: Option<MemberBox>,
}

/// A group of [`GestureArenaMember`] objects that are competing as a unit in the
/// [`crate::GestureArenaManager`].
///
/// Normally, a recognizer competes directly in the [`crate::GestureArenaManager`] to
/// recognize a sequence of pointer events as a gesture. With a
/// [`GestureArenaTeam`], recognizers can compete in the arena in a group with
/// other recognizers. Arena teams may have a captain which wins the arena on
/// behalf of its team.
///
/// When gesture recognizers are in a team together without a captain, then once
/// there are no other competing gestures in the arena, the first gesture to
/// have been added to the team automatically wins, instead of the gestures
/// continuing to compete against each other.
///
/// When gesture recognizers are in a team with a captain, then once one of the
/// team members claims victory or there are no other competing gestures in the
/// arena, the captain wins the arena, and all other team members lose.
#[derive(Clone, Copy)]
pub struct GestureArenaTeam(Handle<GestureArenaTeamData>);

impl GestureArenaTeam {
    /// Creates an empty team.
    pub fn new(app: &mut App) -> GestureArenaTeam {
        GestureArenaTeam(app.create(GestureArenaTeamData {
            combiners: HashMap::new(),
            captain: None,
        }))
    }

    /// A member that wins on behalf of the entire team.
    ///
    /// If not none, when any one of the [`GestureArenaTeam`] members claims victory
    /// the captain accepts the gesture.
    /// If none, the member that claims a victory accepts the gesture.
    pub fn captain(self, app: &App) -> Option<HandleId> {
        app.get(self.0).captain.as_ref().map(MemberBox::member_id)
    }

    /// Sets [`captain`](Self::captain).
    pub fn set_captain(self, app: &mut App, captain: impl GestureArenaMember) {
        app.get_mut(self.0).captain = Some(MemberBox::new(captain));
    }

    /// Adds a new member to the arena on behalf of this team.
    ///
    /// Used by `GestureRecognizer` subclasses that wish to compete in the arena
    /// using this team.
    ///
    /// To assign a gesture recognizer to a team, see
    /// `OneSequenceGestureRecognizer.team`.
    pub fn add(
        self,
        app: &mut App,
        pointer: i64,
        member: impl GestureArenaMember,
    ) -> GestureArenaEntry {
        let member = MemberBox::new(member);
        let combiner = match app.get(self.0).combiners.get(&pointer).copied() {
            Some(combiner) => combiner,
            None => {
                let combiner = CombiningGestureArenaMember(app.create(CombiningData {
                    owner: self,
                    pointer,
                    members: Vec::new(),
                    resolved: false,
                    winner: None,
                    entry: None,
                }));
                app.get_mut(self.0).combiners.insert(pointer, combiner);
                combiner
            }
        };
        combiner.add_member(app, pointer, member)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_foundation::App;

    use crate::binding::GestureBinding;

    const PRIMARY_KEY: i64 = 4;

    struct TestMemberData {
        accept_ran: bool,
        reject_ran: bool,
    }

    #[derive(Clone, Copy)]
    struct TestMember(Handle<TestMemberData>);

    impl TestMember {
        fn new(app: &mut App) -> TestMember {
            TestMember(app.create(TestMemberData {
                accept_ran: false,
                reject_ran: false,
            }))
        }

        fn accept_ran(self, app: &App) -> bool {
            app.get(self.0).accept_ran
        }

        fn reject_ran(self, app: &App) -> bool {
            app.get(self.0).reject_ran
        }
    }

    impl GestureArenaMember for TestMember {
        fn accept_gesture(&self, app: &mut App, _pointer: i64) {
            app.get_mut(self.0).accept_ran = true;
        }

        fn reject_gesture(&self, app: &mut App, _pointer: i64) {
            app.get_mut(self.0).reject_ran = true;
        }

        fn member_id(&self) -> HandleId {
            self.0.id()
        }
    }

    #[test]
    fn first_team_member_wins_when_the_outsider_rejects() {
        let mut app = App::new();
        let binding = GestureBinding::instance(&mut app);
        let team = GestureArenaTeam::new(&mut app);
        let first = TestMember::new(&mut app);
        let second = TestMember::new(&mut app);
        let outsider = TestMember::new(&mut app);

        team.add(&mut app, PRIMARY_KEY, first);
        team.add(&mut app, PRIMARY_KEY, second);
        let outsider_entry = binding
            .gesture_arena(&app)
            .add(&mut app, PRIMARY_KEY, outsider);

        binding.gesture_arena(&app).close(&mut app, PRIMARY_KEY);
        outsider_entry.resolve(&mut app, GestureDisposition::Rejected);
        assert!(!first.accept_ran(&app));
        app.drain_microtasks();
        assert!(first.accept_ran(&app));
        assert!(!first.reject_ran(&app));
        assert!(second.reject_ran(&app));
        assert!(!second.accept_ran(&app));
        assert!(outsider.reject_ran(&app));
    }

    #[test]
    fn first_eager_team_member_wins_without_a_captain() {
        let mut app = App::new();
        let binding = GestureBinding::instance(&mut app);
        let team = GestureArenaTeam::new(&mut app);
        let first = TestMember::new(&mut app);
        let second = TestMember::new(&mut app);

        let first_entry = team.add(&mut app, PRIMARY_KEY, first);
        team.add(&mut app, PRIMARY_KEY, second);
        binding.gesture_arena(&app).close(&mut app, PRIMARY_KEY);
        first_entry.resolve(&mut app, GestureDisposition::Accepted);
        assert!(first.accept_ran(&app));
        assert!(second.reject_ran(&app));
    }

    #[test]
    fn captain_wins_when_a_team_member_accepts() {
        let mut app = App::new();
        let binding = GestureBinding::instance(&mut app);
        let team = GestureArenaTeam::new(&mut app);
        let captain = TestMember::new(&mut app);
        let first = TestMember::new(&mut app);
        let second = TestMember::new(&mut app);

        team.set_captain(&mut app, captain);
        let first_entry = team.add(&mut app, PRIMARY_KEY, first);
        team.add(&mut app, PRIMARY_KEY, second);
        binding.gesture_arena(&app).close(&mut app, PRIMARY_KEY);
        first_entry.resolve(&mut app, GestureDisposition::Accepted);
        assert!(captain.accept_ran(&app));
        assert!(first.reject_ran(&app));
        assert!(second.reject_ran(&app));
    }
}

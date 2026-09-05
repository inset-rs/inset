//! Flutter counterpart: `widgets/shortcuts.dart`.
//!
//! The shortcut system: a [`ShortcutActivator`] describes a key combination, the [`Shortcuts`]
//! widget maps activators to `Intent`s through a [`ShortcutManager`], and [`ShortcutRegistrar`]
//! lets descendants add bindings to a shared [`ShortcutRegistry`].

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::sync::LazyLock;

use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, ListenableObject, Listener,
};
use reveal_scheduler::{FrameCallback, SchedulerBinding};
use reveal_services::{
    HardwareKeyboard, KeyEvent, KeyboardKey, KeyboardLockMode, LogicalKeyboardKey,
};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    StatelessWidget, WidgetRef,
};
use crate::widgets::actions::{Actions, IntentRef};
use crate::widgets::focus_manager::{AnyFocusNode, KeyEventResult, primary_focus};
use crate::widgets::focus_scope::Focus;

static CONTROL_SYNONYMS: LazyLock<HashSet<LogicalKeyboardKey>> = LazyLock::new(|| {
    LogicalKeyboardKey::expand_synonyms(&HashSet::from([LogicalKeyboardKey::CONTROL]))
});
static SHIFT_SYNONYMS: LazyLock<HashSet<LogicalKeyboardKey>> = LazyLock::new(|| {
    LogicalKeyboardKey::expand_synonyms(&HashSet::from([LogicalKeyboardKey::SHIFT]))
});
static ALT_SYNONYMS: LazyLock<HashSet<LogicalKeyboardKey>> = LazyLock::new(|| {
    LogicalKeyboardKey::expand_synonyms(&HashSet::from([LogicalKeyboardKey::ALT]))
});
static META_SYNONYMS: LazyLock<HashSet<LogicalKeyboardKey>> = LazyLock::new(|| {
    LogicalKeyboardKey::expand_synonyms(&HashSet::from([LogicalKeyboardKey::META]))
});

/// A set of `KeyboardKey`s that can be used as the keys in a map.
///
/// A key set contains the keys that are down simultaneously to represent a shortcut.
///
/// This is a thin wrapper around a set, but changes the equality comparison from an identity
/// comparison to a contents comparison so that non-identical sets with the same keys in them
/// will compare as equal.
///
/// See also:
///
///  * [`ShortcutManager`], which uses [`LogicalKeySet`] (a [`KeySet`] wrapper) to define its key
///    map.
#[derive(Clone, PartialEq, Eq)]
pub struct KeySet<T: KeyboardKey + std::hash::Hash> {
    keys: HashSet<T>,
}

impl<T: KeyboardKey + std::hash::Hash> KeySet<T> {
    /// A constructor for making a [`KeySet`] of up to four keys.
    ///
    /// If you need a set of more than four keys, use [`from_set`](Self::from_set).
    ///
    /// The same key may not appear more than once in the set.
    pub fn new(key1: T, key2: Option<T>, key3: Option<T>, key4: Option<T>) -> KeySet<T> {
        let mut keys = HashSet::from([key1]);
        let mut count = 1;
        for key in [key2, key3, key4].into_iter().flatten() {
            keys.insert(key);
            count += 1;
        }
        debug_assert!(
            keys.len() == count,
            "Two or more provided keys are identical. Each key must appear only once."
        );
        KeySet { keys }
    }

    /// Create a [`KeySet`] from a set of keys.
    ///
    /// The `keys` set must not be empty.
    pub fn from_set(keys: HashSet<T>) -> KeySet<T> {
        debug_assert!(!keys.is_empty());
        KeySet { keys }
    }

    /// Returns a copy of the keys in this [`KeySet`].
    pub fn keys(&self) -> HashSet<T> {
        self.keys.clone()
    }
}

impl<T: KeyboardKey + std::hash::Hash> Debug for KeySet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeySet").field("keys", &self.keys).finish()
    }
}

/// Determines how the state of a lock key is used to accept a shortcut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockState {
    /// The lock key state is not used to determine [`SingleActivator::accepts`] result.
    #[default]
    Ignored,

    /// The lock key must be locked to trigger the shortcut.
    Locked,

    /// The lock key must be unlocked to trigger the shortcut.
    Unlocked,
}

/// An interface to define the keyboard key combination to trigger a shortcut.
///
/// [`ShortcutActivator`]s are used by [`Shortcuts`] widgets, and are mapped to `Intent`s, the
/// intended behavior that the key combination should trigger. When a [`Shortcuts`] widget
/// receives a key event, its [`ShortcutManager`] looks up the first matching
/// [`ShortcutActivator`], and signals the corresponding `Intent`, which might trigger an action
/// as defined by a hierarchy of `Actions` widgets.
///
/// The matching [`ShortcutActivator`] is looked up in the following way:
///
///  * Find the registered [`ShortcutActivator`]s whose [`triggers`](Self::triggers) contain the
///    incoming event.
///  * Of the previous list, finds the first activator whose [`accepts`](Self::accepts) returns
///    true in the order of insertion.
///
/// See also:
///
///  * [`SingleActivator`], an implementation that represents a single key combined with
///    modifiers (control, shift, alt, meta).
///  * [`CharacterActivator`], an implementation that represents key combinations that result in
///    the specified character, such as question mark.
///  * [`LogicalKeySet`], an implementation that requires one or more [`LogicalKeyboardKey`]s to
///    be pressed at the same time. Prefer [`SingleActivator`] when possible.
pub trait ShortcutActivator: Debug {
    /// An optional property to provide all the keys that might be the final event to trigger
    /// this shortcut.
    ///
    /// For example, for `Ctrl-A`, `LogicalKeyboardKey::KEY_A` is the only trigger, while
    /// `LogicalKeyboardKey::CONTROL` is not, because the shortcut should only work by pressing
    /// KeyA *after* Ctrl, but not before. For `Ctrl-A-E`, on the other hand, both KeyA and KeyE
    /// should be triggers, since either of them is allowed to trigger.
    ///
    /// If provided, trigger keys can be used as a first-pass filter for incoming events in
    /// order to optimize lookups, as `Intent`s are stored in a map and indexed by trigger keys.
    /// It is up to the individual implementers of this interface to decide if they ignore
    /// triggers or not.
    ///
    /// Implementations should make sure that the return value of this method does not change
    /// throughout the lifespan of this object.
    ///
    /// This method might also return `None`, which means this activator declares all keys as
    /// trigger keys. Activators whose [`triggers`](Self::triggers) return `None` will be tested
    /// with [`accepts`](Self::accepts) on every event. Since this becomes a linear search, and
    /// having too many might impact performance, it is preferred to return a non-`None`
    /// [`triggers`](Self::triggers) whenever possible.
    fn triggers(&self) -> Option<Vec<LogicalKeyboardKey>> {
        None
    }

    /// Whether the triggering `event` and the keyboard `state` at the time of the event meet
    /// required conditions, providing that the event is a triggering event.
    ///
    /// For example, for `Ctrl-A`, it has to check if the event is a key down event, if either
    /// side of the Ctrl key is pressed, and none of the Shift keys, Alt keys, or Meta keys are
    /// pressed; it doesn't have to check if KeyA is pressed, since it's already guaranteed.
    ///
    /// As a possible performance improvement, implementers of this function are encouraged (but
    /// not required) to check the [`triggers`](Self::triggers) member, if it is not `None`, to
    /// see if it contains the event's logical key before doing more complicated work.
    ///
    /// This method must not cause any side effects for the `state`. Typically this is only used
    /// to query whether `HardwareKeyboard::logical_keys_pressed` contains a key.
    fn accepts(&self, app: &App, event: &KeyEvent, state: Handle<HardwareKeyboard>) -> bool;

    /// Returns a description of the key set that is short and readable.
    ///
    /// Intended to be used in debug mode for logging purposes.
    fn debug_describe_keys(&self) -> String;
}

/// The erased [`ShortcutActivator`]: what a shortcut map is keyed by.
pub type ShortcutActivatorRef = Rc<dyn ShortcutActivator>;

/// Dart's `Map<ShortcutActivator, Intent>`: the pairs in insertion order, which is the order
/// [`ShortcutManager`] indexes and searches them in.
pub type ShortcutMap = Vec<(ShortcutActivatorRef, IntentRef)>;

/// A set of [`LogicalKeyboardKey`]s that can be used as the keys in a map.
///
/// [`LogicalKeySet`] can be used as a [`ShortcutActivator`]. It is not recommended to use
/// [`LogicalKeySet`] for a common shortcut such as `Delete` or `Ctrl+C`, prefer
/// [`SingleActivator`] when possible, whose behavior more closely resembles that of typical
/// platforms.
///
/// When used as a [`ShortcutActivator`], [`LogicalKeySet`] will activate the intent when all
/// [`keys`](KeySet::keys) are pressed, and no others, except that modifier keys are considered
/// without considering sides (e.g. control left and control right are considered the same).
///
/// This is also a thin wrapper around a set, but changes the equality comparison from an
/// identity comparison to a contents comparison so that non-identical sets with the same keys
/// in them will compare as equal.
#[derive(Clone, PartialEq, Eq)]
pub struct LogicalKeySet {
    /// Dart's `class LogicalKeySet extends KeySet<LogicalKeyboardKey>`: the superclass's
    /// fields.
    key_set: KeySet<LogicalKeyboardKey>,
    triggers: HashSet<LogicalKeyboardKey>,
}

impl LogicalKeySet {
    /// A constructor for making a [`LogicalKeySet`] of up to four keys.
    ///
    /// If you need a set of more than four keys, use [`from_set`](Self::from_set).
    ///
    /// The same [`LogicalKeyboardKey`] may not appear more than once in the set.
    pub fn new(
        key1: LogicalKeyboardKey,
        key2: Option<LogicalKeyboardKey>,
        key3: Option<LogicalKeyboardKey>,
        key4: Option<LogicalKeyboardKey>,
    ) -> LogicalKeySet {
        LogicalKeySet::of(KeySet::new(key1, key2, key3, key4))
    }

    /// Create a [`LogicalKeySet`] from a set of [`LogicalKeyboardKey`]s.
    pub fn from_set(keys: HashSet<LogicalKeyboardKey>) -> LogicalKeySet {
        LogicalKeySet::of(KeySet::from_set(keys))
    }

    fn of(key_set: KeySet<LogicalKeyboardKey>) -> LogicalKeySet {
        let triggers = key_set
            .keys()
            .into_iter()
            .flat_map(|key| match unmap_synonyms(key) {
                Some(keys) => keys,
                None => vec![key],
            })
            .collect();
        LogicalKeySet { key_set, triggers }
    }

    /// The keys this set was built from.
    pub fn keys(&self) -> HashSet<LogicalKeyboardKey> {
        self.key_set.keys()
    }

    fn check_key_requirements(&self, pressed: &HashSet<LogicalKeyboardKey>) -> bool {
        let collapsed_required = LogicalKeyboardKey::collapse_synonyms(&self.keys());
        let collapsed_pressed = LogicalKeyboardKey::collapse_synonyms(pressed);
        collapsed_required.len() == collapsed_pressed.len()
            && collapsed_required.difference(&collapsed_pressed).count() == 0
    }
}

/// Dart's `LogicalKeySet._modifiers`.
fn modifiers() -> [LogicalKeyboardKey; 4] {
    [
        LogicalKeyboardKey::ALT,
        LogicalKeyboardKey::CONTROL,
        LogicalKeyboardKey::META,
        LogicalKeyboardKey::SHIFT,
    ]
}

/// Dart's `LogicalKeySet._unmapSynonyms`.
fn unmap_synonyms(key: LogicalKeyboardKey) -> Option<Vec<LogicalKeyboardKey>> {
    if key == LogicalKeyboardKey::CONTROL {
        return Some(vec![
            LogicalKeyboardKey::CONTROL_LEFT,
            LogicalKeyboardKey::CONTROL_RIGHT,
        ]);
    }
    if key == LogicalKeyboardKey::SHIFT {
        return Some(vec![
            LogicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::SHIFT_RIGHT,
        ]);
    }
    if key == LogicalKeyboardKey::ALT {
        return Some(vec![
            LogicalKeyboardKey::ALT_LEFT,
            LogicalKeyboardKey::ALT_RIGHT,
        ]);
    }
    if key == LogicalKeyboardKey::META {
        return Some(vec![
            LogicalKeyboardKey::META_LEFT,
            LogicalKeyboardKey::META_RIGHT,
        ]);
    }
    None
}

impl Debug for LogicalKeySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogicalKeySet")
            .field("keys", &self.debug_describe_keys())
            .finish()
    }
}

impl ShortcutActivator for LogicalKeySet {
    fn triggers(&self) -> Option<Vec<LogicalKeyboardKey>> {
        Some(self.triggers.iter().copied().collect())
    }

    fn accepts(&self, app: &App, event: &KeyEvent, state: Handle<HardwareKeyboard>) -> bool {
        if !matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            return false;
        }
        self.triggers.contains(&event.logical_key())
            && self.check_key_requirements(&state.logical_keys_pressed(app))
    }

    fn debug_describe_keys(&self) -> String {
        let mut sorted_keys: Vec<LogicalKeyboardKey> = self.keys().into_iter().collect();
        sorted_keys.sort_by(|a, b| {
            // Put the modifiers first. If it has a synonym, then it's something like shiftLeft,
            // altRight, etc.
            let a_is_modifier = !a.synonyms().is_empty() || modifiers().contains(a);
            let b_is_modifier = !b.synonyms().is_empty() || modifiers().contains(b);
            if a_is_modifier && !b_is_modifier {
                return std::cmp::Ordering::Less;
            }
            if b_is_modifier && !a_is_modifier {
                return std::cmp::Ordering::Greater;
            }
            a.debug_name().cmp(&b.debug_name())
        });
        sorted_keys
            .iter()
            .map(|key| format!("{:?}", key.debug_name()))
            .collect::<Vec<String>>()
            .join(" + ")
    }
}

/// A shortcut key combination of a single key and modifiers.
///
/// [`SingleActivator`] implements typical shortcuts such as:
///
///  * ArrowLeft
///  * Shift + Delete
///  * Control + Alt + Meta + Shift + A
///
/// More specifically, it creates shortcut key combinations that are composed of a
/// [`trigger`](Self::trigger) key, and zero, some, or all of the four modifiers (control,
/// shift, alt, meta). The shortcut is activated when the following conditions are met:
///
///  * The incoming event is a down event for a [`trigger`](Self::trigger) key.
///  * If [`control`](Self::control) is true, then at least one control key must be held.
///    Otherwise, no control keys must be held.
///  * Similar conditions apply for the [`alt`](Self::alt), [`shift`](Self::shift), and
///    [`meta`](Self::meta) keys.
///
/// This resembles the typical behavior of most operating systems, and handles modifier keys
/// differently from [`LogicalKeySet`] in the following way:
///
///  * [`SingleActivator`]s allow additional non-modifier keys being pressed in order to
///    activate the shortcut.
///  * [`SingleActivator`]s do not consider modifiers to be a trigger key.
///
/// See also:
///
///  * [`CharacterActivator`], an activator that represents key combinations that result in the
///    specified character, such as question mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SingleActivator {
    /// The non-modifier key of the shortcut that is pressed after all modifiers to activate the
    /// shortcut.
    ///
    /// For example, for `Control + C`, [`trigger`](Self::trigger) should be
    /// `LogicalKeyboardKey::KEY_C`.
    pub trigger: LogicalKeyboardKey,

    /// Whether either (or both) control keys should be held for [`trigger`](Self::trigger) to
    /// activate the shortcut.
    ///
    /// It defaults to false, meaning all Control keys must be released when the event is
    /// received in order to activate the shortcut. If it's true, then either or both Control
    /// keys must be pressed.
    pub control: bool,

    /// Whether either (or both) shift keys should be held for [`trigger`](Self::trigger) to
    /// activate the shortcut.
    ///
    /// It defaults to false, meaning all Shift keys must be released when the event is received
    /// in order to activate the shortcut. If it's true, then either or both Shift keys must be
    /// pressed.
    pub shift: bool,

    /// Whether either (or both) alt keys should be held for [`trigger`](Self::trigger) to
    /// activate the shortcut.
    ///
    /// It defaults to false, meaning all Alt keys must be released when the event is received
    /// in order to activate the shortcut. If it's true, then either or both Alt keys must be
    /// pressed.
    pub alt: bool,

    /// Whether either (or both) meta keys should be held for [`trigger`](Self::trigger) to
    /// activate the shortcut.
    ///
    /// It defaults to false, meaning all Meta keys must be released when the event is received
    /// in order to activate the shortcut. If it's true, then either or both Meta keys must be
    /// pressed.
    pub meta: bool,

    /// Whether the NumLock key state should be checked for [`trigger`](Self::trigger) to
    /// activate the shortcut.
    ///
    /// It defaults to [`LockState::Ignored`], meaning the NumLock state is ignored when the
    /// event is received in order to activate the shortcut. If it's [`LockState::Locked`], then
    /// the NumLock key must be locked. If it's [`LockState::Unlocked`], then the NumLock key
    /// must be unlocked.
    pub num_lock: LockState,

    /// Whether this activator accepts repeat events of the [`trigger`](Self::trigger) key.
    ///
    /// If [`include_repeats`](Self::include_repeats) is true, the activator is checked on all
    /// key down or key repeat events for the [`trigger`](Self::trigger) key. If it is false,
    /// only [`trigger`](Self::trigger) key events which are key down events will be considered.
    pub include_repeats: bool,
}

impl SingleActivator {
    /// Triggered when the [`trigger`](Self::trigger) key is pressed while the modifiers are
    /// held.
    ///
    /// The [`trigger`](Self::trigger) should be the non-modifier key that is pressed after all
    /// the modifiers, such as `LogicalKeyboardKey::KEY_C` as in `Ctrl+C`. It must not be a
    /// modifier key (sided or unsided).
    pub fn new(trigger: LogicalKeyboardKey) -> SingleActivator {
        debug_assert!(
            ![
                LogicalKeyboardKey::CONTROL,
                LogicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::SHIFT,
                LogicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_RIGHT,
                LogicalKeyboardKey::ALT,
                LogicalKeyboardKey::ALT_LEFT,
                LogicalKeyboardKey::ALT_RIGHT,
                LogicalKeyboardKey::META,
                LogicalKeyboardKey::META_LEFT,
                LogicalKeyboardKey::META_RIGHT,
            ]
            .contains(&trigger)
        );
        SingleActivator {
            trigger,
            control: false,
            shift: false,
            alt: false,
            meta: false,
            num_lock: LockState::Ignored,
            include_repeats: true,
        }
    }

    /// Dart `SingleActivator(control:)`.
    pub fn control(mut self, control: bool) -> SingleActivator {
        self.control = control;
        self
    }

    /// Dart `SingleActivator(shift:)`.
    pub fn shift(mut self, shift: bool) -> SingleActivator {
        self.shift = shift;
        self
    }

    /// Dart `SingleActivator(alt:)`.
    pub fn alt(mut self, alt: bool) -> SingleActivator {
        self.alt = alt;
        self
    }

    /// Dart `SingleActivator(meta:)`.
    pub fn meta(mut self, meta: bool) -> SingleActivator {
        self.meta = meta;
        self
    }

    /// Dart `SingleActivator(numLock:)`.
    pub fn num_lock(mut self, num_lock: LockState) -> SingleActivator {
        self.num_lock = num_lock;
        self
    }

    /// Dart `SingleActivator(includeRepeats:)`.
    pub fn include_repeats(mut self, include_repeats: bool) -> SingleActivator {
        self.include_repeats = include_repeats;
        self
    }

    fn should_accept_modifiers(&self, pressed: &HashSet<LogicalKeyboardKey>) -> bool {
        self.control == intersects(pressed, &CONTROL_SYNONYMS)
            && self.shift == intersects(pressed, &SHIFT_SYNONYMS)
            && self.alt == intersects(pressed, &ALT_SYNONYMS)
            && self.meta == intersects(pressed, &META_SYNONYMS)
    }

    fn should_accept_num_lock(&self, app: &App, state: Handle<HardwareKeyboard>) -> bool {
        match self.num_lock {
            LockState::Ignored => true,
            LockState::Locked => state
                .lock_modes_enabled(app)
                .contains(&KeyboardLockMode::NumLock),
            LockState::Unlocked => !state
                .lock_modes_enabled(app)
                .contains(&KeyboardLockMode::NumLock),
        }
    }
}

/// Dart's `pressed.intersection(synonyms).isNotEmpty`.
fn intersects(
    pressed: &HashSet<LogicalKeyboardKey>,
    synonyms: &HashSet<LogicalKeyboardKey>,
) -> bool {
    pressed.intersection(synonyms).next().is_some()
}

impl ShortcutActivator for SingleActivator {
    fn triggers(&self) -> Option<Vec<LogicalKeyboardKey>> {
        Some(vec![self.trigger])
    }

    fn accepts(&self, app: &App, event: &KeyEvent, state: Handle<HardwareKeyboard>) -> bool {
        (matches!(event, KeyEvent::Down(_))
            || (self.include_repeats && matches!(event, KeyEvent::Repeat(_))))
            && self.trigger == event.logical_key()
            && self.should_accept_modifiers(&state.logical_keys_pressed(app))
            && self.should_accept_num_lock(app, state)
    }

    /// Returns a short and readable description of the key combination.
    fn debug_describe_keys(&self) -> String {
        let mut keys: Vec<String> = Vec::new();
        if self.control {
            keys.push("Control".to_string());
        }
        if self.alt {
            keys.push("Alt".to_string());
        }
        if self.meta {
            keys.push("Meta".to_string());
        }
        if self.shift {
            keys.push("Shift".to_string());
        }
        keys.push(
            self.trigger
                .debug_name()
                .unwrap_or_else(|| format!("{:?}", self.trigger)),
        );
        keys.join(" + ")
    }
}

/// A shortcut combination that is triggered by a key event that produces a specific character.
///
/// Keys often produce different characters when combined with modifiers. For example, it might
/// be helpful for the user to bring up a help menu by pressing the question mark ('?'). However,
/// there is no logical key that directly represents a question mark. Although 'Shift+Slash'
/// produces a '?' character on a US keyboard, its logical key is still considered a Slash key,
/// and hard-coding 'Shift+Slash' in this situation is unfriendly to other keyboard layouts.
///
/// For example, `CharacterActivator::new("?")` is triggered when a key combination results in a
/// question mark, which is 'Shift+Slash' on a US keyboard, but 'Shift+Comma' on a French
/// keyboard.
///
/// The [`alt`](Self::alt), [`control`](Self::control), and [`meta`](Self::meta) flags represent
/// whether the respective modifier keys should be held (true) or released (false). They default
/// to false. [`CharacterActivator`] cannot check shifted keys, since the Shift key affects the
/// resulting character, and will accept whether either of the Shift keys are pressed or not, as
/// long as the key event produces the correct character.
///
/// See also:
///
///  * [`SingleActivator`], an activator that represents a single key combined with modifiers,
///    such as `Ctrl+C` or `Ctrl-Right Arrow`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterActivator {
    /// Whether either (or both) Alt keys should be held for the
    /// [`character`](Self::character) to activate the shortcut.
    ///
    /// It defaults to false, meaning all Alt keys must be released when the event is received
    /// in order to activate the shortcut. If it's true, then either one or both Alt keys must
    /// be pressed.
    pub alt: bool,

    /// Whether either (or both) Control keys should be held for the
    /// [`character`](Self::character) to activate the shortcut.
    ///
    /// It defaults to false, meaning all Control keys must be released when the event is
    /// received in order to activate the shortcut. If it's true, then either one or both
    /// Control keys must be pressed.
    pub control: bool,

    /// Whether either (or both) Meta keys should be held for the
    /// [`character`](Self::character) to activate the shortcut.
    ///
    /// It defaults to false, meaning all Meta keys must be released when the event is received
    /// in order to activate the shortcut. If it's true, then either one or both Meta keys must
    /// be pressed.
    pub meta: bool,

    /// Whether this activator accepts repeat events of the [`character`](Self::character).
    ///
    /// If [`include_repeats`](Self::include_repeats) is true, the activator is checked on all
    /// key down and key repeat events for the [`character`](Self::character). If it is false,
    /// only the [`character`](Self::character) events that are key down events will be
    /// considered.
    pub include_repeats: bool,

    /// The character which triggers the shortcut.
    ///
    /// This is typically a single-character string, such as '?' or 'œ', although
    /// [`CharacterActivator`] doesn't check the length of [`character`](Self::character) or
    /// whether it can be matched by any key combination at all. It is case-sensitive, since the
    /// [`character`](Self::character) is directly compared to the character reported by the
    /// platform.
    pub character: String,
}

impl CharacterActivator {
    /// Triggered when the key event yields the given character.
    pub fn new(character: impl Into<String>) -> CharacterActivator {
        CharacterActivator {
            alt: false,
            control: false,
            meta: false,
            include_repeats: true,
            character: character.into(),
        }
    }

    /// Dart `CharacterActivator(alt:)`.
    pub fn alt(mut self, alt: bool) -> CharacterActivator {
        self.alt = alt;
        self
    }

    /// Dart `CharacterActivator(control:)`.
    pub fn control(mut self, control: bool) -> CharacterActivator {
        self.control = control;
        self
    }

    /// Dart `CharacterActivator(meta:)`.
    pub fn meta(mut self, meta: bool) -> CharacterActivator {
        self.meta = meta;
        self
    }

    /// Dart `CharacterActivator(includeRepeats:)`.
    pub fn include_repeats(mut self, include_repeats: bool) -> CharacterActivator {
        self.include_repeats = include_repeats;
        self
    }

    fn should_accept_modifiers(&self, pressed: &HashSet<LogicalKeyboardKey>) -> bool {
        // Doesn't look for shift, since the character will encode that.
        self.control == intersects(pressed, &CONTROL_SYNONYMS)
            && self.alt == intersects(pressed, &ALT_SYNONYMS)
            && self.meta == intersects(pressed, &META_SYNONYMS)
    }
}

impl ShortcutActivator for CharacterActivator {
    fn accepts(&self, app: &App, event: &KeyEvent, state: Handle<HardwareKeyboard>) -> bool {
        // Ignore triggers, since we're only interested in the character.
        event.character() == Some(self.character.as_str())
            && (matches!(event, KeyEvent::Down(_))
                || (self.include_repeats && matches!(event, KeyEvent::Repeat(_))))
            && self.should_accept_modifiers(&state.logical_keys_pressed(app))
    }

    fn debug_describe_keys(&self) -> String {
        let mut keys: Vec<String> = Vec::new();
        if self.alt {
            keys.push("Alt".to_string());
        }
        if self.control {
            keys.push("Control".to_string());
        }
        if self.meta {
            keys.push("Meta".to_string());
        }
        keys.push(format!("'{}'", self.character));
        keys.join(" + ")
    }
}

/// Dart's `_ActivatorIntentPair`: one entry of a manager's shortcut index. Its fields are
/// private, as Dart's are.
pub struct ActivatorIntentPair {
    activator: ShortcutActivatorRef,
    intent: IntentRef,
}

// ---------------------------------------------------------------------------------------------
// ShortcutManager

/// The fields Dart's `ShortcutManager` declares; a manager carries this bag under the field
/// `shortcut_manager` ([`shortcut_manager_accessors!`](crate::shortcut_manager_accessors)).
pub struct ShortcutManagerData {
    modal: bool,
    shortcuts: ShortcutMap,
    indexed_shortcuts_cache:
        Option<Rc<HashMap<Option<LogicalKeyboardKey>, Vec<ActivatorIntentPair>>>>,
}

impl ShortcutManagerData {
    /// The bag of a freshly created manager.
    pub fn new() -> ShortcutManagerData {
        ShortcutManagerData {
            modal: false,
            shortcuts: Vec::new(),
            indexed_shortcuts_cache: None,
        }
    }
}

impl Default for ShortcutManagerData {
    fn default() -> ShortcutManagerData {
        ShortcutManagerData::new()
    }
}

/// The accessors [`ShortcutManagerBase`] asks for, for a struct whose bag is the field
/// `shortcut_manager`.
#[macro_export]
macro_rules! shortcut_manager_accessors {
    () => {
        fn shortcut_manager_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::ShortcutManagerData {
            &app.get(self).shortcut_manager
        }

        fn shortcut_manager_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::ShortcutManagerData {
            &mut app.get_mut(self).shortcut_manager
        }
    };
}

/// The members Dart's `ShortcutManager` declares, over a [`ShortcutManagerData`] bag.
///
/// Dart's class is both instantiated ([`ShortcutManager`]) and subclassed (`RadioGroup`'s
/// manager ignores a key event while no radio in its group has focus), so the bodies live on
/// this trait and each Dart class is a leaf struct that implements it.
pub trait ShortcutManagerBase: ChangeNotifier + Sized + 'static {
    /// Dart's `ShortcutManager` fields, held under the field `shortcut_manager`
    /// ([`shortcut_manager_accessors!`](crate::shortcut_manager_accessors)).
    fn shortcut_manager_data(self: Handle<Self>, app: &App) -> &ShortcutManagerData;

    /// See [`shortcut_manager_data`](Self::shortcut_manager_data).
    fn shortcut_manager_data_mut(self: Handle<Self>, app: &mut App) -> &mut ShortcutManagerData;

    /// This manager as the erased [`AnyShortcutManager`] — what to pass where a Dart API takes
    /// a `ShortcutManager`.
    fn as_shortcut_manager(self: Handle<Self>) -> AnyShortcutManager {
        AnyShortcutManager {
            id: self.id(),
            vtable: const { &ShortcutManagerVTable::of::<Self>() },
        }
    }

    /// True if the [`ShortcutManagerBase`] should not pass on keys that it doesn't handle to
    /// any key-handling widgets that are ancestors to this one.
    ///
    /// Setting [`modal`](Self::modal) to true will prevent any key event given to this manager
    /// from being given to any ancestor managers, even if that key doesn't appear in the
    /// [`shortcuts`](Self::shortcuts) map.
    ///
    /// The net effect of setting [`modal`](Self::modal) to true is to return
    /// [`KeyEventResult::SkipRemainingHandlers`] from
    /// [`handle_keypress`](Self::handle_keypress) if it does not exist in the shortcut map,
    /// instead of returning [`KeyEventResult::Ignored`].
    fn modal(self: Handle<Self>, app: &App) -> bool {
        self.shortcut_manager_data(app).modal
    }

    /// Dart `ShortcutManager(modal:)`.
    fn set_modal(self: Handle<Self>, app: &mut App, modal: bool) {
        self.shortcut_manager_data_mut(app).modal = modal;
    }

    /// Returns the shortcut map.
    ///
    /// When the map is changed, listeners to this manager will be notified.
    fn shortcuts(self: Handle<Self>, app: &App) -> ShortcutMap {
        self.shortcut_manager_data(app).shortcuts.clone()
    }

    /// See [`shortcuts`](Self::shortcuts).
    fn set_shortcuts(self: Handle<Self>, app: &mut App, value: ShortcutMap) {
        if !same_shortcuts(&self.shortcut_manager_data(app).shortcuts, &value) {
            let data = self.shortcut_manager_data_mut(app);
            data.shortcuts = value;
            data.indexed_shortcuts_cache = None;
            self.notify_listeners(app);
        }
    }

    /// Dart's `ShortcutManager._indexedShortcuts`.
    fn indexed_shortcuts(
        self: Handle<Self>,
        app: &mut App,
    ) -> Rc<HashMap<Option<LogicalKeyboardKey>, Vec<ActivatorIntentPair>>> {
        if let Some(cache) = self
            .shortcut_manager_data(app)
            .indexed_shortcuts_cache
            .clone()
        {
            return cache;
        }
        let indexed = Rc::new(index_shortcuts(&self.shortcut_manager_data(app).shortcuts));
        self.shortcut_manager_data_mut(app).indexed_shortcuts_cache = Some(Rc::clone(&indexed));
        indexed
    }

    /// Returns the `Intent`, if any, that matches the current set of pressed keys.
    ///
    /// Returns `None` if no intent matches the current set of pressed keys.
    fn find(
        self: Handle<Self>,
        app: &mut App,
        event: &KeyEvent,
        state: Handle<HardwareKeyboard>,
    ) -> Option<IntentRef> {
        let indexed = self.indexed_shortcuts(app);
        let key = Some(event.logical_key());
        let candidates = indexed
            .get(&key)
            .into_iter()
            .flatten()
            .chain(indexed.get(&None).into_iter().flatten());
        for activator_intent in candidates {
            if activator_intent.activator.accepts(app, event, state) {
                return Some(Rc::clone(&activator_intent.intent));
            }
        }
        None
    }

    /// Handles a key press `event` in the given `context`.
    ///
    /// If a key mapping is found, then the associated action will be invoked using the `Intent`
    /// activated by the [`ShortcutActivator`] in the [`shortcuts`](Self::shortcuts) map, and the
    /// currently focused widget's context (from `FocusManager::primary_focus`).
    ///
    /// Returns [`KeyEventResult::Handled`] if an action was invoked, otherwise
    /// [`KeyEventResult::SkipRemainingHandlers`] if [`modal`](Self::modal) is true, or if it
    /// maps to a `DoNothingAction` with `consumes_key` set to false, and in all other cases
    /// returns [`KeyEventResult::Ignored`].
    ///
    /// In order for an action to be invoked (and [`KeyEventResult::Handled`] returned), a
    /// [`ShortcutActivator`] must accept the given `KeyEvent`, be mapped to an `Intent`, the
    /// `Intent` must be mapped to an `Action`, and the `Action` must be enabled.
    fn handle_keypress(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        event: &KeyEvent,
    ) -> KeyEventResult {
        ShortcutManagerKeypress::handle_keypress(self, app, context, event)
    }
}

/// Flutter's `ShortcutManager.handleKeypress` body that an override calls through `super`.
///
/// A trait default cannot call `super`, and a leaf's own
/// [`ShortcutManagerBase::handle_keypress`] shadows the default it would call; this sibling
/// trait, blanket-implemented for every manager, carries that body (the shape `ElementBase` has
/// for `Element`).
pub trait ShortcutManagerKeypress: ShortcutManagerBase {
    /// Flutter's `ShortcutManager.handleKeypress`.
    fn handle_keypress(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        event: &KeyEvent,
    ) -> KeyEventResult {
        // Dart shadows the `context` parameter with the primary focus's own context.
        let _ = context;
        let keyboard = HardwareKeyboard::instance(app);
        let intent = self.find(app, event, keyboard);
        let context = primary_focus(app).and_then(|node| node.context(app));

        if let Some(intent) = intent
            && let Some(context) = context
            && let Some(action) = Actions::maybe_find(app, context, intent.intent_type())
        {
            let (enabled, invoke_result) = Actions::of(app, context).invoke_action_if_enabled(
                app,
                action,
                &*intent,
                Some(context),
            );
            if enabled {
                return action.to_key_event_result(app, &*intent, invoke_result.as_ref());
            }
        }
        if self.modal(app) {
            KeyEventResult::SkipRemainingHandlers
        } else {
            KeyEventResult::Ignored
        }
    }
}

impl<T: ShortcutManagerBase> ShortcutManagerKeypress for T {}

/// The vtable of an erased [`AnyShortcutManager`]: one `&'static` table per leaf type.
struct ShortcutManagerVTable {
    type_name: fn() -> &'static str,
    shortcuts: fn(&App, HandleId) -> ShortcutMap,
    set_shortcuts: fn(&mut App, HandleId, ShortcutMap),
    modal: fn(&App, HandleId) -> bool,
    set_modal: fn(&mut App, HandleId, bool),
    handle_keypress: fn(&mut App, HandleId, BuildContext, &KeyEvent) -> KeyEventResult,
    dispose: fn(&mut App, HandleId),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

impl ShortcutManagerVTable {
    const fn of<L: ShortcutManagerBase>() -> ShortcutManagerVTable {
        ShortcutManagerVTable {
            type_name: std::any::type_name::<L>,
            shortcuts: |app, id| L::shortcuts(resolve(id), app),
            set_shortcuts: |app, id, value| L::set_shortcuts(resolve(id), app, value),
            modal: |app, id| L::modal(resolve(id), app),
            set_modal: |app, id, value| L::set_modal(resolve(id), app, value),
            handle_keypress: |app, id, context, event| {
                L::handle_keypress(resolve(id), app, context, event)
            },
            dispose: |app, id| {
                let handle: Handle<L> = resolve(id);
                app.get_mut(handle).change_notifier_data_mut().dispose();
            },
        }
    }
}

/// Erased `ShortcutManager`: one identity and a static vtable.
///
/// This is what a field or parameter Dart types as `ShortcutManager` becomes.
#[derive(Clone, Copy)]
pub struct AnyShortcutManager {
    id: HandleId,
    vtable: &'static ShortcutManagerVTable,
}

impl PartialEq for AnyShortcutManager {
    fn eq(&self, other: &AnyShortcutManager) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyShortcutManager {}

impl fmt::Debug for AnyShortcutManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyShortcutManager {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `manager as T`: the typed handle when this manager is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// See [`ShortcutManagerBase::shortcuts`].
    pub fn shortcuts(self, app: &App) -> ShortcutMap {
        (self.vtable.shortcuts)(app, self.id)
    }

    /// See [`ShortcutManagerBase::set_shortcuts`].
    pub fn set_shortcuts(self, app: &mut App, value: ShortcutMap) {
        (self.vtable.set_shortcuts)(app, self.id, value);
    }

    /// See [`ShortcutManagerBase::modal`].
    pub fn modal(self, app: &App) -> bool {
        (self.vtable.modal)(app, self.id)
    }

    /// See [`ShortcutManagerBase::set_modal`].
    pub fn set_modal(self, app: &mut App, value: bool) {
        (self.vtable.set_modal)(app, self.id, value);
    }

    /// See [`ShortcutManagerBase::handle_keypress`].
    pub fn handle_keypress(
        self,
        app: &mut App,
        context: BuildContext,
        event: &KeyEvent,
    ) -> KeyEventResult {
        (self.vtable.handle_keypress)(app, self.id, context, event)
    }

    /// `ChangeNotifier.dispose` on the manager behind this handle.
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id);
    }
}

/// A manager of keyboard shortcut bindings used by [`Shortcuts`] to handle key events.
///
/// The manager may be listened to (with `add_listener` / `remove_listener`) for change
/// notifications when the shortcuts change.
///
/// Typically, a [`Shortcuts`] widget supplies its own manager, but in uncommon cases where
/// overriding the usual shortcut manager behavior is desired, a custom [`ShortcutManagerBase`]
/// may be supplied.
pub struct ShortcutManager {
    change_notifier: ChangeNotifierData,
    shortcut_manager: ShortcutManagerData,
}

impl ShortcutManager {
    /// Constructs a [`ShortcutManager`].
    ///
    /// Dart's optional `shortcuts` and `modal` arguments are the setters.
    pub fn new(app: &mut App) -> Handle<ShortcutManager> {
        app.create(ShortcutManager {
            change_notifier: ChangeNotifierData::new(),
            shortcut_manager: ShortcutManagerData::new(),
        })
    }
}

impl ShortcutManagerBase for ShortcutManager {
    crate::shortcut_manager_accessors!();
}

impl ChangeNotifier for ShortcutManager {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// Dart's `ShortcutManager._indexShortcuts`.
fn index_shortcuts(
    source: &ShortcutMap,
) -> HashMap<Option<LogicalKeyboardKey>, Vec<ActivatorIntentPair>> {
    let mut result: HashMap<Option<LogicalKeyboardKey>, Vec<ActivatorIntentPair>> = HashMap::new();
    for (activator, intent) in source {
        let nullable_triggers = activator.triggers();
        let triggers: Vec<Option<LogicalKeyboardKey>> = match nullable_triggers {
            Some(triggers) => triggers.into_iter().map(Some).collect(),
            None => vec![None],
        };
        for trigger in triggers {
            result
                .entry(trigger)
                .or_default()
                .push(ActivatorIntentPair {
                    activator: Rc::clone(activator),
                    intent: Rc::clone(intent),
                });
        }
    }
    result
}

/// Dart's `mapEquals` on a shortcut map, whose keys and values are reference types.
fn same_shortcuts(a: &ShortcutMap, b: &ShortcutMap) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(a, b)| Rc::ptr_eq(&a.0, &b.0) && Rc::ptr_eq(&a.1, &b.1))
}

// ---------------------------------------------------------------------------------------------
// Shortcuts

/// A widget that creates key bindings to specific actions for its descendants.
///
/// This widget establishes a [`ShortcutManager`] to be used by its descendants when invoking an
/// `Action` via a keyboard key combination that maps to an `Intent`.
///
/// This is similar to but more powerful than the [`CallbackShortcuts`] widget. Unlike
/// [`CallbackShortcuts`], this widget separates key bindings and their implementations. This
/// separation allows [`Shortcuts`] to have key bindings that adapt to the focused context.
///
/// See also:
///
///  * [`CallbackShortcuts`], a simpler but less flexible widget that defines key bindings that
///    invoke callbacks.
///  * `Intent`, a trait for values containing a description of a user action to be invoked.
///  * `Action`, a trait for defining an invocation of a user action.
///  * `CallbackAction`, a class for creating an action from a callback.
pub struct Shortcuts {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The [`ShortcutManagerBase`] that will manage the mapping between key combinations and
    /// `Action`s.
    ///
    /// If this widget was created with [`Shortcuts::manager`], then
    /// [`ShortcutManagerBase::shortcuts`] will be used as the source for shortcuts. If
    /// [`Shortcuts::new`] is used, this manager will be `None`, and a default-constructed
    /// [`ShortcutManager`] will be used.
    pub manager: Option<AnyShortcutManager>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// The debug label that is printed for this node when logged.
    ///
    /// If this label is set, then it will be displayed instead of the shortcut map when logged.
    pub debug_label: Option<String>,

    /// Whether to include semantics from `Focus`.
    ///
    /// Defaults to true.
    pub include_semantics: bool,

    /// Dart's `Shortcuts._shortcuts`.
    shortcuts: ShortcutMap,
}

impl Shortcuts {
    /// Creates a [`Shortcuts`] widget that owns the map of shortcuts and creates its own
    /// manager.
    ///
    /// When using this constructor, [`manager`](Self::manager) is `None`.
    ///
    /// See also:
    ///
    ///  * [`Shortcuts::manager`], a constructor that uses a [`ShortcutManagerBase`] to manage
    ///    the shortcuts list instead.
    pub fn new<K>(shortcuts: ShortcutMap, child: impl IntoWidget<K>) -> Shortcuts {
        Shortcuts {
            key: None,
            manager: None,
            child: child.into_widget(),
            debug_label: None,
            include_semantics: true,
            shortcuts,
        }
    }

    /// Creates a [`Shortcuts`] widget that uses the `manager` to manage the map of shortcuts.
    ///
    /// If this constructor is used, [`get_shortcuts`](Self::get_shortcuts) will return the
    /// contents of [`ShortcutManagerBase::shortcuts`].
    pub fn manager<K>(manager: AnyShortcutManager, child: impl IntoWidget<K>) -> Shortcuts {
        Shortcuts {
            manager: Some(manager),
            ..Shortcuts::new(Vec::new(), child)
        }
    }

    /// Dart `Shortcuts(key:)`.
    pub fn key(mut self, key: KeyRef) -> Shortcuts {
        self.key = Some(key);
        self
    }

    /// Dart `Shortcuts(debugLabel:)`.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> Shortcuts {
        self.debug_label = Some(debug_label.into());
        self
    }

    /// Dart `Shortcuts(includeSemantics:)`.
    pub fn include_semantics(mut self, include_semantics: bool) -> Shortcuts {
        self.include_semantics = include_semantics;
        self
    }

    /// The map of shortcuts that describes the mapping between a key sequence defined by a
    /// [`ShortcutActivator`] and the `Intent` that will be emitted when that key sequence is
    /// pressed.
    pub fn get_shortcuts(&self, app: &App) -> ShortcutMap {
        match self.manager {
            None => self.shortcuts.clone(),
            Some(manager) => manager.shortcuts(app),
        }
    }
}

impl fmt::Debug for Shortcuts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shortcuts")
            .field("manager", &self.manager)
            .field("debugLabel", &self.debug_label)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Shortcuts {
    type State = ShortcutsState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ShortcutsState {
        ShortcutsState {
            state: StateData::new(),
            internal_manager: None,
        }
    }
}

/// Dart's `_ShortcutsState`.
pub struct ShortcutsState {
    state: StateData<Shortcuts>,
    internal_manager: Option<Handle<ShortcutManager>>,
}

impl ShortcutsState {
    /// The manager in effect: the widget's, or the one this state created.
    pub fn manager(self: Handle<Self>, app: &App) -> AnyShortcutManager {
        self.widget(app)
            .manager
            .or_else(|| {
                app.get(self)
                    .internal_manager
                    .map(ShortcutManagerBase::as_shortcut_manager)
            })
            .expect("a Shortcuts state has either the widget's manager or its own")
    }

    fn handle_on_key_event(
        self: Handle<Self>,
        app: &mut App,
        node: AnyFocusNode,
        event: &KeyEvent,
    ) -> KeyEventResult {
        let Some(context) = node.context(app) else {
            return KeyEventResult::Ignored;
        };
        self.manager(app).handle_keypress(app, context, event)
    }
}

impl State for ShortcutsState {
    type Widget = Shortcuts;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if self.widget(app).manager.is_none() {
            let manager = ShortcutManager::new(app);
            app.get_mut(self).internal_manager = Some(manager);
            let shortcuts = self.widget(app).get_shortcuts(app);
            manager.set_shortcuts(app, shortcuts);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Shortcuts) {
        if self.widget(app).manager != old_widget.manager {
            if self.widget(app).manager.is_some() {
                if let Some(internal) = app.get_mut(self).internal_manager.take() {
                    app.get_mut(internal).change_notifier.dispose();
                }
            } else if app.get(self).internal_manager.is_none() {
                let manager = ShortcutManager::new(app);
                app.get_mut(self).internal_manager = Some(manager);
            }
        }
        if let Some(internal) = app.get(self).internal_manager {
            let shortcuts = self.widget(app).get_shortcuts(app);
            internal.set_shortcuts(app, shortcuts);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(internal) = app.get(self).internal_manager {
            app.get_mut(internal).change_notifier.dispose();
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let debug_label = match &widget.debug_label {
            Some(label) => format!("Shortcuts: {label}"),
            None => "Shortcuts".to_string(),
        };
        let include_semantics = widget.include_semantics;
        let child = widget.child.clone();
        Focus::new(child)
            .debug_label(debug_label)
            .can_request_focus(false)
            .on_key_event(Rc::new(move |app, node, event| {
                self.handle_on_key_event(app, node, event)
            }))
            .include_semantics(include_semantics)
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// CallbackShortcuts

/// A widget that binds key combinations to specific callbacks.
///
/// This is similar to but simpler than the [`Shortcuts`] widget as it doesn't require `Intent`s
/// and `Actions` widgets. Instead, it accepts a map of [`ShortcutActivator`]s to callbacks.
///
/// Unlike [`Shortcuts`], this widget does not separate key bindings and their implementations.
/// That separation allows [`Shortcuts`] to have key bindings that adapt to the focused context.
///
/// See also:
///
///  * [`Shortcuts`], a more powerful widget for defining key bindings.
///  * `Focus`, a widget that defines which widgets can receive keyboard focus.
pub struct CallbackShortcuts {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// A map of key combinations to callbacks used to define the shortcut bindings.
    ///
    /// If a descendant of this widget has focus, and a key is pressed, the activator keys of
    /// this map will be asked if they accept the key event. If they do, then the corresponding
    /// callback is invoked, and the key event propagation is halted. If none of the activators
    /// accept the key event, then the key event continues to be propagated up the focus chain.
    ///
    /// If more than one activator accepts the key event, then all of the callbacks associated
    /// with activators that accept the key event are invoked.
    pub bindings: Vec<(ShortcutActivatorRef, Listener)>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl CallbackShortcuts {
    /// Creates a [`CallbackShortcuts`] widget.
    pub fn new<K>(
        bindings: Vec<(ShortcutActivatorRef, Listener)>,
        child: impl IntoWidget<K>,
    ) -> CallbackShortcuts {
        CallbackShortcuts {
            key: None,
            bindings,
            child: child.into_widget(),
        }
    }

    /// Dart `CallbackShortcuts(key:)`.
    pub fn key(mut self, key: KeyRef) -> CallbackShortcuts {
        self.key = Some(key);
        self
    }

    /// A helper function to make the stack trace more useful if the callback panics, by
    /// providing the activator and event as arguments that will appear in the stack trace.
    fn apply_key_event_binding(
        app: &mut App,
        binding: &(ShortcutActivatorRef, Listener),
        event: &KeyEvent,
    ) -> bool {
        let keyboard = HardwareKeyboard::instance(app);
        if binding.0.accepts(app, event, keyboard) {
            binding.1.call(app);
            return true;
        }
        false
    }
}

impl fmt::Debug for CallbackShortcuts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackShortcuts").finish_non_exhaustive()
    }
}

impl StatelessWidget for CallbackShortcuts {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let bindings: Rc<Vec<(ShortcutActivatorRef, Listener)>> = Rc::new(
            self.bindings
                .iter()
                .map(|(activator, callback)| (Rc::clone(activator), callback.clone()))
                .collect(),
        );
        Focus::new(self.child.clone())
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _node, event| {
                let mut result = KeyEventResult::Ignored;
                // Activates all key bindings that match, returns "handled" if any handle it.
                for binding in bindings.iter() {
                    result = if CallbackShortcuts::apply_key_event_binding(app, binding, event) {
                        KeyEventResult::Handled
                    } else {
                        result
                    };
                }
                result
            }))
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// ShortcutRegistry

/// An entry returned by [`ShortcutRegistry::add_all`] that allows the caller to identify the
/// shortcuts they registered with the [`ShortcutRegistry`] through the [`ShortcutRegistrar`].
///
/// When the entry is no longer needed, [`dispose`](Self::dispose) should be called, and the
/// entry should no longer be used.
pub struct ShortcutRegistryEntry {
    /// The [`ShortcutRegistry`] that this entry was issued by.
    registry: Handle<ShortcutRegistry>,
}

impl ShortcutRegistryEntry {
    /// The [`ShortcutRegistry`] that this entry was issued by.
    pub fn registry(self: Handle<Self>, app: &App) -> Handle<ShortcutRegistry> {
        app.get(self).registry
    }

    /// Replaces the given shortcut bindings in the [`ShortcutRegistry`] that this entry was
    /// created from.
    ///
    /// It will assert if this entry has already been disposed.
    ///
    /// If two equivalent, but different, [`ShortcutActivator`]s are added, all of them will be
    /// executed when triggered.
    pub fn replace_all(self: Handle<Self>, app: &mut App, value: ShortcutMap) {
        let registry = app.get(self).registry;
        registry.replace_all(app, self, value);
    }

    /// Called when the entry is no longer needed.
    ///
    /// Calling this will remove all shortcuts associated with this [`ShortcutRegistryEntry`]
    /// from the [`registry`](Self::registry).
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let registry = app.get(self).registry;
        registry.dispose_entry(app, self);
    }
}

/// A class used by [`ShortcutRegistrar`] that allows adding or removing shortcut bindings by
/// descendants of the [`ShortcutRegistrar`].
///
/// You can reach the nearest [`ShortcutRegistry`] using [`of`](Self::of) and
/// [`maybe_of`](Self::maybe_of).
///
/// The registry may be listened to (with `add_listener` / `remove_listener`) for change
/// notifications when the registered shortcuts change. Change notifications take place after the
/// current frame is drawn, so that widgets that are not descendants of the registry can listen
/// to it.
pub struct ShortcutRegistry {
    change_notifier: ChangeNotifierData,
    notification_scheduled: bool,
    disposed: bool,
    /// Dart's `_registeredShortcuts`, in insertion order: that is the order
    /// [`shortcuts`](Self::shortcuts) merges the entries in.
    registered_shortcuts: Vec<(Handle<ShortcutRegistryEntry>, ShortcutMap)>,
}

impl ShortcutRegistry {
    /// Creates an instance of [`ShortcutRegistry`].
    pub fn new(app: &mut App) -> Handle<ShortcutRegistry> {
        app.create(ShortcutRegistry {
            change_notifier: ChangeNotifierData::new(),
            notification_scheduled: false,
            disposed: false,
            registered_shortcuts: Vec::new(),
        })
    }

    /// Discards any resources used by this object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let this = app.get_mut(self);
        this.change_notifier.dispose();
        this.disposed = true;
    }

    /// Gets the combined shortcut bindings from all contexts that are registered with this
    /// [`ShortcutRegistry`].
    ///
    /// Listeners will be notified when the value returned by this getter changes.
    ///
    /// Returns a copy: modifying the returned map will have no effect.
    pub fn shortcuts(self: Handle<Self>, app: &App) -> ShortcutMap {
        debug_assert!(ChangeNotifierData::debug_assert_not_disposed(
            app.get(self).change_notifier_data()
        ));
        app.get(self)
            .registered_shortcuts
            .iter()
            .flat_map(|(_, shortcuts)| shortcuts.iter().cloned())
            .collect()
    }

    /// Adds all the given shortcut bindings to this [`ShortcutRegistry`], and returns an entry
    /// for managing those bindings.
    ///
    /// [`ShortcutRegistryEntry::dispose`] should be called on the entry when these shortcuts are
    /// no longer needed. This will remove them from the registry, and invalidate the entry.
    ///
    /// If two equivalent, but different, [`ShortcutActivator`]s are added, all of them will be
    /// executed when triggered.
    ///
    /// See also:
    ///
    ///  * [`ShortcutRegistryEntry::replace_all`], a function used to replace the set of
    ///    shortcuts associated with a particular entry.
    ///  * [`ShortcutRegistryEntry::dispose`], a function used to remove the set of shortcuts
    ///    associated with a particular entry.
    pub fn add_all(
        self: Handle<Self>,
        app: &mut App,
        value: ShortcutMap,
    ) -> Handle<ShortcutRegistryEntry> {
        debug_assert!(ChangeNotifierData::debug_assert_not_disposed(
            app.get(self).change_notifier_data()
        ));
        debug_assert!(
            !value.is_empty(),
            "Cannot register an empty map of shortcuts"
        );
        let entry = app.create(ShortcutRegistryEntry { registry: self });
        app.get_mut(self).registered_shortcuts.push((entry, value));
        self.notify_listeners_next_frame(app);
        entry
    }

    /// Subscriber notification has to happen in the next frame because shortcuts are often
    /// registered that affect things in the overlay or different parts of the tree, and so can
    /// cause build ordering issues if notifications happen during the build. The
    /// `notification_scheduled` check makes sure we only notify once per frame.
    fn notify_listeners_next_frame(self: Handle<Self>, app: &mut App) {
        if !app.get(self).notification_scheduled {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _duration| {
                    app.get_mut(self).notification_scheduled = false;
                    if !app.get(self).disposed {
                        self.notify_listeners(app);
                    }
                }),
            );
            app.get_mut(self).notification_scheduled = true;
        }
    }

    /// Returns the [`ShortcutRegistry`] that belongs to the [`ShortcutRegistrar`] which most
    /// tightly encloses the given [`BuildContext`].
    ///
    /// If no [`ShortcutRegistrar`] widget encloses the context given, [`of`](Self::of) panics.
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), which is similar to this function, but will return
    ///    `None` if it doesn't find a [`ShortcutRegistrar`] ancestor.
    pub fn of(app: &mut App, context: BuildContext) -> Handle<ShortcutRegistry> {
        ShortcutRegistry::maybe_of(app, context).expect(
            "Unable to find a ShortcutRegistrar widget in the context.\n\
             ShortcutRegistrar.of() was called with a context that does not contain a \
             ShortcutRegistrar widget.",
        )
    }

    /// Returns the [`ShortcutRegistry`] of the [`ShortcutRegistrar`] that most tightly encloses
    /// the given [`BuildContext`].
    ///
    /// If no [`ShortcutRegistrar`] widget encloses the given context, [`maybe_of`](Self::maybe_of)
    /// will return `None`.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<ShortcutRegistry>> {
        context
            .depend_on_inherited_widget_of_exact_type::<ShortcutRegistrarScope>(app)
            .map(|inherited| inherited.registry)
    }

    /// Replaces all the shortcuts associated with the given entry from this registry.
    fn replace_all(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<ShortcutRegistryEntry>,
        value: ShortcutMap,
    ) {
        debug_assert!(ChangeNotifierData::debug_assert_not_disposed(
            app.get(self).change_notifier_data()
        ));
        debug_assert!(self.debug_check_entry_is_valid(app, entry));
        for registered in app.get_mut(self).registered_shortcuts.iter_mut() {
            if registered.0 == entry {
                registered.1 = value;
                break;
            }
        }
        self.notify_listeners_next_frame(app);
    }

    /// Removes all the shortcuts associated with the given entry from this registry.
    fn dispose_entry(self: Handle<Self>, app: &mut App, entry: Handle<ShortcutRegistryEntry>) {
        debug_assert!(self.debug_check_entry_is_valid(app, entry));
        let registered = &mut app.get_mut(self).registered_shortcuts;
        let before = registered.len();
        registered.retain(|(registered, _)| *registered != entry);
        if app.get(self).registered_shortcuts.len() != before {
            self.notify_listeners_next_frame(app);
        }
    }

    fn debug_check_entry_is_valid(
        self: Handle<Self>,
        app: &App,
        entry: Handle<ShortcutRegistryEntry>,
    ) -> bool {
        if !app
            .get(self)
            .registered_shortcuts
            .iter()
            .any(|(registered, _)| *registered == entry)
        {
            assert!(
                app.get(entry).registry != self,
                "entry {entry:?} is invalid.\nThe entry has already been disposed of. Tokens \
                 are not valid after dispose is called on them, and should no longer be used."
            );
            panic!(
                "Foreign entry {entry:?} used.\nThis entry was not created by this registry, \
                 it was created by {:?}, and should be used with that registry instead.",
                app.get(entry).registry
            );
        }
        true
    }
}

impl ChangeNotifier for ShortcutRegistry {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// A widget that holds a [`ShortcutRegistry`] which allows descendants to add, remove, or
/// replace shortcuts.
///
/// This widget holds a [`ShortcutRegistry`] so that its descendants can find it with
/// [`ShortcutRegistry::of`] or [`ShortcutRegistry::maybe_of`].
///
/// The registered shortcuts are valid whenever a widget below this one in the hierarchy has
/// focus.
///
/// To add shortcuts to the registry, call [`ShortcutRegistry::of`] or
/// [`ShortcutRegistry::maybe_of`] to get the [`ShortcutRegistry`], and then add them using
/// [`ShortcutRegistry::add_all`], which will return a [`ShortcutRegistryEntry`] which must be
/// disposed by calling [`ShortcutRegistryEntry::dispose`] when the shortcuts are no longer
/// needed.
#[derive(Debug)]
pub struct ShortcutRegistrar {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl ShortcutRegistrar {
    /// Creates a [`ShortcutRegistrar`].
    pub fn new<K>(child: impl IntoWidget<K>) -> ShortcutRegistrar {
        ShortcutRegistrar {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `ShortcutRegistrar(key:)`.
    pub fn key(mut self, key: KeyRef) -> ShortcutRegistrar {
        self.key = Some(key);
        self
    }
}

impl StatefulWidget for ShortcutRegistrar {
    type State = ShortcutRegistrarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ShortcutRegistrarState {
        ShortcutRegistrarState {
            state: StateData::new(),
            registry: None,
            manager: None,
            shortcuts_changed: None,
        }
    }
}

/// Dart's `_ShortcutRegistrarState`.
pub struct ShortcutRegistrarState {
    state: StateData<ShortcutRegistrar>,
    registry: Option<Handle<ShortcutRegistry>>,
    manager: Option<Handle<ShortcutManager>>,
    /// Dart's `_shortcutsChanged` tear-off, kept so that it can be removed again.
    shortcuts_changed: Option<Listener>,
}

impl ShortcutRegistrarState {
    /// The registry this state owns.
    pub fn registry(self: Handle<Self>, app: &App) -> Handle<ShortcutRegistry> {
        app.get(self).registry.expect("created in init_state")
    }

    /// The manager this state owns.
    pub fn manager(self: Handle<Self>, app: &App) -> Handle<ShortcutManager> {
        app.get(self).manager.expect("created in init_state")
    }

    fn shortcuts_changed(self: Handle<Self>, app: &mut App) {
        // This shouldn't need to update the widget, and avoids calling set_state during the
        // build phase.
        let registry = self.registry(app);
        let shortcuts = registry.shortcuts(app);
        self.manager(app).set_shortcuts(app, shortcuts);
    }
}

impl State for ShortcutRegistrarState {
    type Widget = ShortcutRegistrar;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let registry = ShortcutRegistry::new(app);
        let manager = ShortcutManager::new(app);
        let this = app.get_mut(self);
        this.registry = Some(registry);
        this.manager = Some(manager);
        let listener = Listener::handle_method(self, ShortcutRegistrarState::shortcuts_changed);
        app.get_mut(self).shortcuts_changed = Some(listener.clone());
        registry.add_listener(app, listener);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let registry = self.registry(app);
        if let Some(listener) = app.get_mut(self).shortcuts_changed.take() {
            registry.remove_listener(app, &listener);
        }
        registry.dispose(app);
        let manager = self.manager(app);
        app.get_mut(manager).change_notifier.dispose();
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let registry = self.registry(app);
        let manager = self.manager(app);
        let child = self.widget(app).child.clone();
        ShortcutRegistrarScope {
            registry,
            child: Shortcuts::manager(manager.as_shortcut_manager(), child)
                .debug_label("<Shortcut Registrar>")
                .into_widget(),
        }
        .into_widget()
    }
}

/// Dart's `_ShortcutRegistrarScope`.
#[derive(Debug)]
struct ShortcutRegistrarScope {
    registry: Handle<ShortcutRegistry>,
    child: WidgetRef,
}

impl InheritedWidget for ShortcutRegistrarScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ShortcutRegistrarScope) -> bool {
        self.registry != old_widget.registry
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};
    use std::cell::RefCell;
    use std::time::Duration;

    use reveal_services::{KeyDownEvent, KeyUpEvent, PhysicalKeyboardKey};

    use super::*;
    use crate::widgets::actions::{Action, AnyAction, CallbackAction, Intent};
    use crate::widgets::basic::SizedBox;
    use crate::widgets::focus_manager::tests::{app_with_view, mount};

    #[derive(Debug)]
    struct SaveIntent;

    impl Intent for SaveIntent {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn key_down(physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) -> KeyEvent {
        KeyEvent::Down(KeyDownEvent::new(physical, logical, Duration::ZERO))
    }

    fn key_up(physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) -> KeyEvent {
        KeyEvent::Up(KeyUpEvent::new(physical, logical, Duration::ZERO))
    }

    /// A `CallbackAction` for [`SaveIntent`] that records that it ran.
    fn save_action(app: &mut App, log: &Rc<RefCell<Vec<&'static str>>>) -> AnyAction {
        let log = Rc::clone(log);
        let action = CallbackAction::<SaveIntent>::new(
            app,
            Rc::new(move |_app, _intent| {
                log.borrow_mut().push("saved");
                None
            }),
        );
        Action::as_action(action)
    }

    #[test]
    fn a_shortcut_maps_a_key_event_to_an_intent_and_invokes_its_action() {
        let mut app = app_with_view();
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::default();
        let action = save_action(&mut app, &log);
        let shortcuts: ShortcutMap = vec![(
            Rc::new(SingleActivator::new(LogicalKeyboardKey::KEY_S).control(true)),
            Rc::new(SaveIntent),
        )];
        mount(
            &mut app,
            Shortcuts::new(
                shortcuts,
                Actions::new(
                    HashMap::from([(TypeId::of::<SaveIntent>(), action)]),
                    Focus::new(SizedBox::shrink()).autofocus(true),
                ),
            )
            .into_widget(),
        );

        let keyboard = HardwareKeyboard::instance(&mut app);
        // Without the modifier the activator does not accept the event.
        assert!(!keyboard.handle_key_event(
            &mut app,
            &key_down(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S)
        ));
        assert!(log.borrow().is_empty());
        keyboard.handle_key_event(
            &mut app,
            &key_up(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S),
        );

        keyboard.handle_key_event(
            &mut app,
            &key_down(
                PhysicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_LEFT,
            ),
        );
        assert!(keyboard.handle_key_event(
            &mut app,
            &key_down(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S)
        ));
        assert_eq!(*log.borrow(), ["saved"]);
    }

    #[test]
    fn a_logical_key_set_activates_on_the_whole_combination() {
        let mut app = app_with_view();
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::default();
        let action = save_action(&mut app, &log);
        let shortcuts: ShortcutMap = vec![(
            Rc::new(LogicalKeySet::new(
                LogicalKeyboardKey::CONTROL,
                Some(LogicalKeyboardKey::KEY_S),
                None,
                None,
            )),
            Rc::new(SaveIntent),
        )];
        mount(
            &mut app,
            Shortcuts::new(
                shortcuts,
                Actions::new(
                    HashMap::from([(TypeId::of::<SaveIntent>(), action)]),
                    Focus::new(SizedBox::shrink()).autofocus(true),
                ),
            )
            .into_widget(),
        );

        let keyboard = HardwareKeyboard::instance(&mut app);
        keyboard.handle_key_event(
            &mut app,
            &key_down(
                PhysicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_LEFT,
            ),
        );
        assert!(keyboard.handle_key_event(
            &mut app,
            &key_down(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S)
        ));
        assert_eq!(*log.borrow(), ["saved"]);
    }

    #[test]
    fn callback_shortcuts_call_every_binding_that_accepts_the_event() {
        let mut app = app_with_view();
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::default();
        let first = Rc::clone(&log);
        let second = Rc::clone(&log);
        let bindings: Vec<(ShortcutActivatorRef, Listener)> = vec![
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::KEY_A)),
                Listener::new(move |_app| first.borrow_mut().push("first")),
            ),
            (
                Rc::new(CharacterActivator::new("a")),
                Listener::new(move |_app| second.borrow_mut().push("second")),
            ),
        ];
        mount(
            &mut app,
            CallbackShortcuts::new(bindings, Focus::new(SizedBox::shrink()).autofocus(true))
                .into_widget(),
        );

        let keyboard = HardwareKeyboard::instance(&mut app);
        let event = KeyEvent::Down(
            KeyDownEvent::new(
                PhysicalKeyboardKey::KEY_A,
                LogicalKeyboardKey::KEY_A,
                Duration::ZERO,
            )
            .character("a"),
        );
        assert!(keyboard.handle_key_event(&mut app, &event));
        assert_eq!(*log.borrow(), ["first", "second"]);
    }

    #[test]
    fn the_manager_reindexes_and_notifies_when_its_shortcuts_change() {
        let mut app = app_with_view();
        mount(&mut app, SizedBox::shrink().into_widget());
        let manager = ShortcutManager::new(&mut app);
        let notifications: Rc<RefCell<u32>> = Rc::default();
        let sink = Rc::clone(&notifications);
        manager.add_listener(&mut app, Listener::new(move |_app| *sink.borrow_mut() += 1));

        let activator: ShortcutActivatorRef =
            Rc::new(SingleActivator::new(LogicalKeyboardKey::KEY_S));
        let intent: IntentRef = Rc::new(SaveIntent);
        let shortcuts: ShortcutMap = vec![(Rc::clone(&activator), Rc::clone(&intent))];
        manager.set_shortcuts(&mut app, shortcuts.clone());
        assert_eq!(*notifications.borrow(), 1);

        // The same pairs are not a change.
        manager.set_shortcuts(&mut app, shortcuts);
        assert_eq!(*notifications.borrow(), 1);

        let keyboard = HardwareKeyboard::instance(&mut app);
        let found = manager.find(
            &mut app,
            &key_down(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S),
            keyboard,
        );
        assert!(found.is_some_and(|found| Rc::ptr_eq(&found, &intent)));
    }

    #[test]
    fn the_registry_merges_its_entries_into_the_registrars_manager() {
        let mut app = app_with_view();
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::default();
        let action = save_action(&mut app, &log);
        let captured: Rc<std::cell::Cell<Option<BuildContext>>> = Rc::default();
        let sink = Rc::clone(&captured);
        mount(
            &mut app,
            ShortcutRegistrar::new(Actions::new(
                HashMap::from([(TypeId::of::<SaveIntent>(), action)]),
                crate::widgets::basic::Builder::new(move |_app, context| {
                    sink.set(Some(context));
                    Focus::new(SizedBox::shrink()).autofocus(true).into_widget()
                }),
            ))
            .into_widget(),
        );
        let context = captured.get().expect("the builder ran");

        let registry = ShortcutRegistry::of(&mut app, context);
        let entry = registry.add_all(
            &mut app,
            vec![(
                Rc::new(SingleActivator::new(LogicalKeyboardKey::KEY_S)),
                Rc::new(SaveIntent),
            )],
        );
        // The registry notifies its listeners after the frame the entry was added in.
        crate::widgets::focus_manager::tests::pump_frame(&mut app);

        let keyboard = HardwareKeyboard::instance(&mut app);
        assert!(keyboard.handle_key_event(
            &mut app,
            &key_down(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S)
        ));
        assert_eq!(*log.borrow(), ["saved"]);

        entry.dispose(&mut app);
        crate::widgets::focus_manager::tests::pump_frame(&mut app);
        log.borrow_mut().clear();
        keyboard.handle_key_event(
            &mut app,
            &key_up(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S),
        );
        assert!(!keyboard.handle_key_event(
            &mut app,
            &key_down(PhysicalKeyboardKey::KEY_S, LogicalKeyboardKey::KEY_S)
        ));
        assert!(log.borrow().is_empty());
    }
}

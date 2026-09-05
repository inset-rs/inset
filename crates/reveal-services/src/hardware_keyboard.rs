//! Flutter counterpart: `services/hardware_keyboard.dart`.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{KeyData, KeyEventDeviceType, KeyEventType};
use reveal_foundation::{App, Handle};

use crate::keyboard_key::{LogicalKeyboardKey, PhysicalKeyboardKey};

/// Represents a lock mode of a keyboard, such as [`KeyboardLockMode::CapsLock`].
///
/// A lock mode locks some of a keyboard's keys into a distinct mode of operation,
/// depending on the lock settings selected. The status of the mode is toggled
/// with each key down of its corresponding logical key. A [`KeyboardLockMode`]
/// object is used to query whether this mode is enabled on the keyboard.
///
/// Only a limited number of modes are supported, which are enumerated as
/// members of this enum. Manual constructing of this type is prohibited.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardLockMode {
    /// Represents the number lock mode on the keyboard.
    ///
    /// On supporting systems, enabling number lock mode usually allows key
    /// presses of the number pad to input numbers, instead of acting as up, down,
    /// left, right, page up, end, etc.
    NumLock,

    /// Represents the scrolling lock mode on the keyboard.
    ///
    /// On supporting systems and applications (such as a spreadsheet), enabling
    /// scrolling lock mode usually allows key presses of the cursor keys to
    /// scroll the document instead of the cursor.
    ScrollLock,

    /// Represents the capital letters lock mode on the keyboard.
    ///
    /// On supporting systems, enabling capital lock mode allows key presses of
    /// the letter keys to input uppercase letters instead of lowercase.
    CapsLock,
}

impl KeyboardLockMode {
    /// The logical key that triggers this lock mode.
    pub fn logical_key(&self) -> LogicalKeyboardKey {
        match self {
            KeyboardLockMode::NumLock => LogicalKeyboardKey::NUM_LOCK,
            KeyboardLockMode::ScrollLock => LogicalKeyboardKey::SCROLL_LOCK,
            KeyboardLockMode::CapsLock => LogicalKeyboardKey::CAPS_LOCK,
        }
    }

    /// Returns the [`KeyboardLockMode`] constant from the logical key, or
    /// `None`, if not found.
    pub fn find_lock_by_logical_key(logical_key: LogicalKeyboardKey) -> Option<KeyboardLockMode> {
        KNOWN_LOCK_MODES
            .iter()
            .copied()
            .find(|lock_mode| lock_mode.logical_key() == logical_key)
    }
}

/// Dart's `KeyboardLockMode._knownLockModes`.
static KNOWN_LOCK_MODES: &[KeyboardLockMode] = &[
    KeyboardLockMode::NumLock,
    KeyboardLockMode::ScrollLock,
    KeyboardLockMode::CapsLock,
];

/// An event indicating that the user has pressed a key down on the keyboard.
///
/// See also:
///
///  * [`KeyRepeatEvent`], a key event representing the user holding a key,
///    causing repeated events.
///  * [`KeyUpEvent`], a key event representing the user releasing a key.
///  * [`HardwareKeyboard`], which produces this event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyDownEvent {
    /// See [`KeyEvent::physical_key`].
    pub physical_key: PhysicalKeyboardKey,

    /// See [`KeyEvent::logical_key`].
    pub logical_key: LogicalKeyboardKey,

    /// See [`KeyEvent::character`].
    pub character: Option<String>,

    /// See [`KeyEvent::time_stamp`].
    pub time_stamp: Duration,

    /// See [`KeyEvent::device_type`].
    pub device_type: KeyEventDeviceType,

    /// See [`KeyEvent::synthesized`].
    pub synthesized: bool,
}

impl KeyDownEvent {
    /// Creates a key event that represents the user pressing a key.
    pub fn new(
        physical_key: PhysicalKeyboardKey,
        logical_key: LogicalKeyboardKey,
        time_stamp: Duration,
    ) -> KeyDownEvent {
        KeyDownEvent {
            physical_key,
            logical_key,
            character: None,
            time_stamp,
            device_type: KeyEventDeviceType::Keyboard,
            synthesized: false,
        }
    }

    /// Dart `KeyDownEvent(character:)`.
    pub fn character(mut self, character: impl Into<String>) -> KeyDownEvent {
        self.character = Some(character.into());
        self
    }

    /// Dart `KeyDownEvent(deviceType:)`.
    pub fn device_type(mut self, device_type: KeyEventDeviceType) -> KeyDownEvent {
        self.device_type = device_type;
        self
    }

    /// Dart `KeyDownEvent(synthesized:)`.
    pub fn synthesized(mut self, synthesized: bool) -> KeyDownEvent {
        self.synthesized = synthesized;
        self
    }
}

/// An event indicating that the user has released a key on the keyboard.
///
/// See also:
///
///  * [`KeyDownEvent`], a key event representing the user pressing a key.
///  * [`KeyRepeatEvent`], a key event representing the user holding a key,
///    causing repeated events.
///  * [`HardwareKeyboard`], which produces this event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyUpEvent {
    /// See [`KeyEvent::physical_key`].
    pub physical_key: PhysicalKeyboardKey,

    /// See [`KeyEvent::logical_key`].
    pub logical_key: LogicalKeyboardKey,

    /// See [`KeyEvent::time_stamp`].
    pub time_stamp: Duration,

    /// See [`KeyEvent::device_type`].
    pub device_type: KeyEventDeviceType,

    /// See [`KeyEvent::synthesized`].
    pub synthesized: bool,
}

impl KeyUpEvent {
    /// Creates a key event that represents the user releasing a key.
    pub fn new(
        physical_key: PhysicalKeyboardKey,
        logical_key: LogicalKeyboardKey,
        time_stamp: Duration,
    ) -> KeyUpEvent {
        KeyUpEvent {
            physical_key,
            logical_key,
            time_stamp,
            device_type: KeyEventDeviceType::Keyboard,
            synthesized: false,
        }
    }

    /// Dart `KeyUpEvent(deviceType:)`.
    pub fn device_type(mut self, device_type: KeyEventDeviceType) -> KeyUpEvent {
        self.device_type = device_type;
        self
    }

    /// Dart `KeyUpEvent(synthesized:)`.
    pub fn synthesized(mut self, synthesized: bool) -> KeyUpEvent {
        self.synthesized = synthesized;
        self
    }
}

/// An event indicating that the user has been holding a key on the keyboard
/// and causing repeated events.
///
/// Repeat events are not guaranteed and are provided only if supported by the
/// underlying platform.
///
/// See also:
///
///  * [`KeyDownEvent`], a key event representing the user pressing a key.
///  * [`KeyUpEvent`], a key event representing the user releasing a key.
///  * [`HardwareKeyboard`], which produces this event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRepeatEvent {
    /// See [`KeyEvent::physical_key`].
    pub physical_key: PhysicalKeyboardKey,

    /// See [`KeyEvent::logical_key`].
    pub logical_key: LogicalKeyboardKey,

    /// See [`KeyEvent::character`].
    pub character: Option<String>,

    /// See [`KeyEvent::time_stamp`].
    pub time_stamp: Duration,

    /// See [`KeyEvent::device_type`].
    pub device_type: KeyEventDeviceType,
}

impl KeyRepeatEvent {
    /// Creates a key event that represents the user holding a key.
    pub fn new(
        physical_key: PhysicalKeyboardKey,
        logical_key: LogicalKeyboardKey,
        time_stamp: Duration,
    ) -> KeyRepeatEvent {
        KeyRepeatEvent {
            physical_key,
            logical_key,
            character: None,
            time_stamp,
            device_type: KeyEventDeviceType::Keyboard,
        }
    }

    /// Dart `KeyRepeatEvent(character:)`.
    pub fn character(mut self, character: impl Into<String>) -> KeyRepeatEvent {
        self.character = Some(character.into());
        self
    }

    /// Dart `KeyRepeatEvent(deviceType:)`.
    pub fn device_type(mut self, device_type: KeyEventDeviceType) -> KeyRepeatEvent {
        self.device_type = device_type;
        self
    }
}

/// Defines the interface for keyboard key events.
///
/// The [`KeyEvent`] provides a universal model for key event information from a
/// hardware keyboard across platforms.
///
/// See also:
///
///  * [`HardwareKeyboard`] for full introduction to key event model and handling.
///  * [`KeyDownEvent`], the variant for events representing the user pressing a
///    key.
///  * [`KeyRepeatEvent`], the variant for events representing the user holding a
///    key, causing repeated events.
///  * [`KeyUpEvent`], the variant for events representing the user releasing a
///    key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    /// [`KeyDownEvent`].
    Down(KeyDownEvent),
    /// [`KeyUpEvent`].
    Up(KeyUpEvent),
    /// [`KeyRepeatEvent`].
    Repeat(KeyRepeatEvent),
}

impl KeyEvent {
    /// Returns an object representing the physical location of this key.
    ///
    /// A [`PhysicalKeyboardKey`] represents a USB HID code sent from the keyboard,
    /// ignoring the key map, modifier keys (like SHIFT), and the label on the key.
    ///
    /// [`PhysicalKeyboardKey`]s are used to describe and test for keys in a
    /// particular location. A [`PhysicalKeyboardKey`] may have a name, but the name
    /// is a mnemonic ("keyA" is easier to remember than 0x70004), derived from the
    /// key's effect on a QWERTY keyboard. The name does not represent the key's
    /// effect whatsoever (a physical "keyA" can be the Q key on an AZERTY
    /// keyboard).
    ///
    /// For instance, if you wanted to make a game where the key to the right of
    /// the CAPS LOCK key made the player move left, you would be comparing a
    /// physical key with [`PhysicalKeyboardKey::KEY_A`], since that is the key next
    /// to the CAPS LOCK key on a QWERTY keyboard. This would return the same thing
    /// even on an AZERTY keyboard where the key next to the CAPS LOCK produces a
    /// "Q" when pressed.
    ///
    /// If you want to make your app respond to a key with a particular character
    /// on it regardless of location of the key, use
    /// [`logical_key`](Self::logical_key) instead.
    ///
    /// Also, even though physical keys are defined with USB HID codes, their
    /// values are not necessarily the same HID codes produced by the hardware and
    /// presented to the driver. On most platforms, Flutter has to map the
    /// platform representation back to a HID code because the original HID
    /// code is not provided.
    pub fn physical_key(&self) -> PhysicalKeyboardKey {
        match self {
            KeyEvent::Down(event) => event.physical_key,
            KeyEvent::Up(event) => event.physical_key,
            KeyEvent::Repeat(event) => event.physical_key,
        }
    }

    /// Returns an object representing the logical key that was pressed.
    ///
    /// This method takes into account the key map and modifier keys (like SHIFT)
    /// to determine which logical key to return.
    ///
    /// If you are looking for the character produced by a key event, use
    /// [`character`](Self::character) instead.
    pub fn logical_key(&self) -> LogicalKeyboardKey {
        match self {
            KeyEvent::Down(event) => event.logical_key,
            KeyEvent::Up(event) => event.logical_key,
            KeyEvent::Repeat(event) => event.logical_key,
        }
    }

    /// Returns the Unicode character (grapheme cluster) completed by this
    /// keystroke, if any.
    ///
    /// This will only return a character if this keystroke, combined with any
    /// preceding keystroke(s), generates a character, and only on a "key down"
    /// event. It will return `None` if no character has been generated by the
    /// keystroke (e.g. a "dead" or "combining" key), or if the corresponding key
    /// is a key without a visual representation, such as a modifier key or a
    /// control key. It will also return `None` if this is a "key up" event.
    ///
    /// This can return multiple Unicode code points, since some characters (more
    /// accurately referred to as grapheme clusters) are made up of more than one
    /// code point.
    ///
    /// The character doesn't take into account edits by an input method editor
    /// (IME), or manage the visibility of the soft keyboard on touch devices.
    ///
    /// The character is not available on [`KeyUpEvent`]s.
    pub fn character(&self) -> Option<&str> {
        match self {
            KeyEvent::Down(event) => event.character.as_deref(),
            KeyEvent::Up(_) => None,
            KeyEvent::Repeat(event) => event.character.as_deref(),
        }
    }

    /// Time of event, relative to an arbitrary start point.
    ///
    /// All events share the same time stamp origin.
    pub fn time_stamp(&self) -> Duration {
        match self {
            KeyEvent::Down(event) => event.time_stamp,
            KeyEvent::Up(event) => event.time_stamp,
            KeyEvent::Repeat(event) => event.time_stamp,
        }
    }

    /// The source device type for the key event.
    ///
    /// Not all platforms supply an accurate type.
    ///
    /// Defaults to [`KeyEventDeviceType::Keyboard`].
    pub fn device_type(&self) -> KeyEventDeviceType {
        match self {
            KeyEvent::Down(event) => event.device_type,
            KeyEvent::Up(event) => event.device_type,
            KeyEvent::Repeat(event) => event.device_type,
        }
    }

    /// Whether this event is synthesized by Flutter to synchronize key states.
    ///
    /// A non-synthesized event is converted from a native event, and a native
    /// event can only be converted to one non-synthesized event. Some properties
    /// might be changed during the conversion (for example, a native repeat event
    /// might be converted to a Flutter down event when necessary.)
    ///
    /// A synthesized event is created without a source native event in order to
    /// synchronize key states. For example, if the native platform shows that a
    /// shift key that was previously held has been released somehow without the
    /// key up event dispatched (probably due to loss of focus), a synthesized key
    /// up event will be added to regularized the event stream.
    ///
    /// For detailed introduction to the regularized event model, see
    /// [`HardwareKeyboard`].
    ///
    /// A [`KeyRepeatEvent`] is never synthesized.
    pub fn synthesized(&self) -> bool {
        match self {
            KeyEvent::Down(event) => event.synthesized,
            KeyEvent::Up(event) => event.synthesized,
            KeyEvent::Repeat(_) => false,
        }
    }
}

impl From<KeyDownEvent> for KeyEvent {
    fn from(event: KeyDownEvent) -> KeyEvent {
        KeyEvent::Down(event)
    }
}

impl From<KeyUpEvent> for KeyEvent {
    fn from(event: KeyUpEvent) -> KeyEvent {
        KeyEvent::Up(event)
    }
}

impl From<KeyRepeatEvent> for KeyEvent {
    fn from(event: KeyRepeatEvent) -> KeyEvent {
        KeyEvent::Repeat(event)
    }
}

/// The signature for [`HardwareKeyboard::add_handler`], a callback to decide
/// whether the entire framework handles a key event.
///
/// [`HardwareKeyboard::remove_handler`] matches the handler by allocation
/// (`Rc::ptr_eq`), where Dart matches the function by identity.
pub type KeyEventCallback = Rc<dyn Fn(&mut App, &KeyEvent) -> bool>;

/// Manages key events from hardware keyboards.
///
/// [`HardwareKeyboard`] manages all key events of the application from hardware
/// keyboards (in contrast to on-screen keyboards). It receives key data from the
/// native platform, dispatches key events to registered handlers, and records the
/// keyboard state.
///
/// To stay notified whenever keys are pressed, held, or released, add a handler
/// with [`add_handler`](Self::add_handler). Handlers should be removed with
/// [`remove_handler`](Self::remove_handler) when notification is no longer
/// necessary, or when the handler is being disposed.
///
/// To query whether a key is being held, or a lock mode is enabled, use
/// [`physical_keys_pressed`](Self::physical_keys_pressed),
/// [`logical_keys_pressed`](Self::logical_keys_pressed), or
/// [`lock_modes_enabled`](Self::lock_modes_enabled). These states will have been
/// updated with the event when used during a key event handler.
///
/// ## Event model
///
/// Flutter uses a universal event model ([`KeyEvent`]) and key options
/// ([`LogicalKeyboardKey`] and [`PhysicalKeyboardKey`]) regardless of the native
/// platform, while preserving platform-specific features as much as possible.
///
/// [`HardwareKeyboard`] guarantees that the key model is "regularized": The key
/// event stream consists of "key tap sequences", where a key tap sequence is
/// defined as one [`KeyDownEvent`], zero or more [`KeyRepeatEvent`]s, and one
/// [`KeyUpEvent`] in order, all with the same physical key and logical key.
///
/// Example:
///
///  * Tap and hold key A, US layout:
///     * `KeyDownEvent(physicalKey: keyA, logicalKey: keyA, character: "a")`
///     * `KeyRepeatEvent(physicalKey: keyA, logicalKey: keyA, character: "a")`
///     * `KeyUpEvent(physicalKey: keyA, logicalKey: keyA)`
///  * Press ShiftLeft, tap key A, then release ShiftLeft, US layout:
///     * `KeyDownEvent(physicalKey: shiftLeft, logicalKey: shiftLeft)`
///     * `KeyDownEvent(physicalKey: keyA, logicalKey: keyA, character: "A")`
///     * `KeyUpEvent(physicalKey: keyA, logicalKey: keyA)`
///     * `KeyUpEvent(physicalKey: shiftLeft, logicalKey: shiftLeft)`
///
/// When the application starts, all keys are released, and all lock modes are
/// disabled. Upon key events, [`HardwareKeyboard`] will update its states, then
/// dispatch callbacks: [`KeyDownEvent`]s and [`KeyUpEvent`]s set or reset the
/// pressing state, while [`KeyDownEvent`]s also toggle lock modes.
///
/// Flutter will try to synchronize with the ground truth of keyboard states
/// using synthesized events ([`KeyEvent::synthesized`]), subject to the
/// availability of the platform. The desynchronization can be caused by
/// non-empty initial state or a change in the focused window or application.
///
/// Flutter does not distinguish between multiple keyboards. Flutter will
/// process all events as if they come from a single keyboard, and try to
/// resolve any conflicts and provide a regularized key event stream, which
/// can deviate from the ground truth.
///
/// Dart's `HardwareKeyboard.instance` is [`instance`](Self::instance), the App's
/// singleton.
#[derive(Default)]
pub struct HardwareKeyboard {
    pressed_keys: HashMap<PhysicalKeyboardKey, LogicalKeyboardKey>,
    lock_modes: HashSet<KeyboardLockMode>,
    handlers: Vec<KeyEventCallback>,
    during_dispatch: bool,
    modified_handlers: Option<Vec<KeyEventCallback>>,
}

impl HardwareKeyboard {
    /// The current [`HardwareKeyboard`], if one has been created.
    ///
    /// Dart's `HardwareKeyboard.instance`.
    pub fn instance(app: &mut App) -> Handle<HardwareKeyboard> {
        app.singleton::<HardwareKeyboard>()
    }

    /// The set of [`PhysicalKeyboardKey`]s that are pressed.
    ///
    /// If called from a key event handler, the result will already include the
    /// effect of the event.
    ///
    /// See also:
    ///
    ///  * [`logical_keys_pressed`](Self::logical_keys_pressed), which tells if a
    ///    logical key is being pressed.
    pub fn physical_keys_pressed(self: Handle<Self>, app: &App) -> HashSet<PhysicalKeyboardKey> {
        app.get(self).pressed_keys.keys().copied().collect()
    }

    /// The set of [`LogicalKeyboardKey`]s that are pressed.
    ///
    /// If called from a key event handler, the result will already include the
    /// effect of the event.
    ///
    /// See also:
    ///
    ///  * [`physical_keys_pressed`](Self::physical_keys_pressed), which tells if a
    ///    physical key is being pressed.
    pub fn logical_keys_pressed(self: Handle<Self>, app: &App) -> HashSet<LogicalKeyboardKey> {
        app.get(self).pressed_keys.values().copied().collect()
    }

    /// Returns the logical key that corresponds to the given pressed physical key.
    ///
    /// Returns `None` if the physical key is not currently pressed.
    pub fn look_up_layout(
        self: Handle<Self>,
        app: &App,
        physical_key: PhysicalKeyboardKey,
    ) -> Option<LogicalKeyboardKey> {
        app.get(self).pressed_keys.get(&physical_key).copied()
    }

    /// The set of [`KeyboardLockMode`] that are enabled.
    ///
    /// Lock keys, such as CapsLock, are logical keys that toggle their
    /// respective boolean states on key down events. Such flags are usually used
    /// as modifier to other keys or events.
    ///
    /// If called from a key event handler, the result will already include the
    /// effect of the event.
    pub fn lock_modes_enabled(self: Handle<Self>, app: &App) -> HashSet<KeyboardLockMode> {
        app.get(self).lock_modes.clone()
    }

    /// Returns true if the given [`LogicalKeyboardKey`] is pressed, according to
    /// the [`HardwareKeyboard`].
    pub fn is_logical_key_pressed(self: Handle<Self>, app: &App, key: LogicalKeyboardKey) -> bool {
        app.get(self)
            .pressed_keys
            .values()
            .any(|&pressed| pressed == key)
    }

    /// Returns true if the given [`PhysicalKeyboardKey`] is pressed, according to
    /// the [`HardwareKeyboard`].
    pub fn is_physical_key_pressed(
        self: Handle<Self>,
        app: &App,
        key: PhysicalKeyboardKey,
    ) -> bool {
        app.get(self).pressed_keys.contains_key(&key)
    }

    /// Returns true if a logical CTRL modifier key is pressed, regardless of
    /// which side of the keyboard it is on.
    ///
    /// Use [`is_logical_key_pressed`](Self::is_logical_key_pressed) if you need to
    /// know which control key was pressed.
    pub fn is_control_pressed(self: Handle<Self>, app: &App) -> bool {
        self.is_logical_key_pressed(app, LogicalKeyboardKey::CONTROL_LEFT)
            || self.is_logical_key_pressed(app, LogicalKeyboardKey::CONTROL_RIGHT)
    }

    /// Returns true if a logical SHIFT modifier key is pressed, regardless of
    /// which side of the keyboard it is on.
    ///
    /// Use [`is_logical_key_pressed`](Self::is_logical_key_pressed) if you need to
    /// know which shift key was pressed.
    pub fn is_shift_pressed(self: Handle<Self>, app: &App) -> bool {
        self.is_logical_key_pressed(app, LogicalKeyboardKey::SHIFT_LEFT)
            || self.is_logical_key_pressed(app, LogicalKeyboardKey::SHIFT_RIGHT)
    }

    /// Returns true if a logical ALT modifier key is pressed, regardless of which
    /// side of the keyboard it is on.
    ///
    /// The `AltGr` key that appears on some keyboards is considered to be the
    /// same as [`LogicalKeyboardKey::ALT_RIGHT`] on some platforms (notably
    /// Android). On platforms that can distinguish between `altRight` and
    /// `altGr`, a press of `AltGr` will not return true here, and will need to be
    /// tested for separately.
    ///
    /// Use [`is_logical_key_pressed`](Self::is_logical_key_pressed) if you need to
    /// know which alt key was pressed.
    pub fn is_alt_pressed(self: Handle<Self>, app: &App) -> bool {
        self.is_logical_key_pressed(app, LogicalKeyboardKey::ALT_LEFT)
            || self.is_logical_key_pressed(app, LogicalKeyboardKey::ALT_RIGHT)
    }

    /// Returns true if a logical META modifier key is pressed, regardless of
    /// which side of the keyboard it is on.
    ///
    /// Use [`is_logical_key_pressed`](Self::is_logical_key_pressed) if you need to
    /// know which meta key was pressed.
    pub fn is_meta_pressed(self: Handle<Self>, app: &App) -> bool {
        self.is_logical_key_pressed(app, LogicalKeyboardKey::META_LEFT)
            || self.is_logical_key_pressed(app, LogicalKeyboardKey::META_RIGHT)
    }

    /// Register a listener that is called every time a hardware key event
    /// occurs.
    ///
    /// All registered handlers will be invoked in order regardless of
    /// their return value. The return value indicates whether Flutter
    /// "handles" the event. If any handler returns true, the event
    /// will not be propagated to other native components in the add-to-app
    /// scenario.
    ///
    /// If an object added a handler, it must remove the handler before it is
    /// disposed.
    ///
    /// If used during event dispatching, the addition will not take effect
    /// until after the dispatching.
    ///
    /// See also:
    ///
    ///  * [`remove_handler`](Self::remove_handler), which removes the handler.
    pub fn add_handler(self: Handle<Self>, app: &mut App, handler: KeyEventCallback) {
        let keyboard = app.get_mut(self);
        if keyboard.during_dispatch {
            keyboard
                .modified_handlers
                .get_or_insert_with(|| keyboard.handlers.clone())
                .push(handler);
        } else {
            keyboard.handlers.push(handler);
        }
    }

    /// Stop calling the given listener every time a hardware key event
    /// occurs.
    ///
    /// The `handler` argument must be a clone of the one used in
    /// [`add_handler`](Self::add_handler). If multiple exist, the first one will
    /// be removed. If none is found, then this method is a no-op.
    ///
    /// If used during event dispatching, the removal will not take effect
    /// until after the event has been dispatched.
    pub fn remove_handler(self: Handle<Self>, app: &mut App, handler: &KeyEventCallback) {
        let keyboard = app.get_mut(self);
        let handlers = if keyboard.during_dispatch {
            keyboard
                .modified_handlers
                .get_or_insert_with(|| keyboard.handlers.clone())
        } else {
            &mut keyboard.handlers
        };
        if let Some(index) = handlers
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, handler))
        {
            handlers.remove(index);
        }
    }

    /// Update the pressed keys to the state the host reports.
    ///
    /// Both the framework and the host maintain a state of the current pressed
    /// keys. There are edge cases, related to startup and restart, where the
    /// framework needs to resynchronize its keyboard state. `keyboard_state` maps
    /// a USB HID usage to the logical key id pressed on it.
    pub fn sync_keyboard_state(
        self: Handle<Self>,
        app: &mut App,
        keyboard_state: &HashMap<u64, u64>,
    ) {
        let keyboard = app.get_mut(self);
        for (&physical, &logical) in keyboard_state {
            keyboard.pressed_keys.insert(
                PhysicalKeyboardKey::new(physical),
                LogicalKeyboardKey::new(logical),
            );
        }
    }

    fn dispatch_key_event(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> bool {
        // This dispatching could have used the same algorithm as `ChangeNotifier`,
        // but since 1) it shouldn't be necessary to support reentrantly
        // dispatching, 2) there shouldn't be many handlers (most apps should use
        // only 1, this function just uses a simpler algorithm.
        debug_assert!(
            !app.get(self).during_dispatch,
            "Nested keyboard dispatching is not supported"
        );
        app.get_mut(self).during_dispatch = true;
        let mut handled = false;
        // Dart iterates the field; a handler re-enters `App`, so the list is taken
        // out of the arena for the walk.
        let handlers = app.get(self).handlers.clone();
        for handler in handlers {
            let this_result = handler(app, event);
            handled = handled || this_result;
        }
        let keyboard = app.get_mut(self);
        keyboard.during_dispatch = false;
        if let Some(modified_handlers) = keyboard.modified_handlers.take() {
            keyboard.handlers = modified_handlers;
        }
        handled
    }

    /// Process a new [`KeyEvent`] by recording the state changes and dispatching
    /// to handlers.
    ///
    /// Returns true if any handler handled the event.
    pub fn handle_key_event(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> bool {
        let physical_key = event.physical_key();
        let logical_key = event.logical_key();
        match event {
            KeyEvent::Down(_) => {
                app.get_mut(self)
                    .pressed_keys
                    .insert(physical_key, logical_key);
                if let Some(lock_mode) = KeyboardLockMode::find_lock_by_logical_key(logical_key) {
                    let lock_modes = &mut app.get_mut(self).lock_modes;
                    if lock_modes.contains(&lock_mode) {
                        lock_modes.remove(&lock_mode);
                    } else {
                        lock_modes.insert(lock_mode);
                    }
                }
            }
            KeyEvent::Up(_) => {
                app.get_mut(self).pressed_keys.remove(&physical_key);
            }
            KeyEvent::Repeat(_) => {
                // Update the logical key in case it has changed.
                app.get_mut(self)
                    .pressed_keys
                    .insert(physical_key, logical_key);
            }
        }
        self.dispatch_key_event(app, event)
    }

    /// Clear all keyboard states and additional handlers.
    ///
    /// This is used by the testing framework to make sure that tests are hermetic.
    pub fn clear_state(self: Handle<Self>, app: &mut App) {
        let keyboard = app.get_mut(self);
        keyboard.pressed_keys.clear();
        keyboard.lock_modes.clear();
        keyboard.handlers.clear();
        debug_assert!(keyboard.modified_handlers.is_none());
    }
}

/// A singleton class that processes key messages from the platform and
/// dispatches converted messages accordingly.
///
/// [`KeyEventManager`] receives platform key messages by
/// [`handle_key_data`](Self::handle_key_data), and sends converted events to
/// [`HardwareKeyboard`] for record keeping and dispatching.
///
/// Flutter owns this from `ServicesBinding`; here it is the App's singleton.
#[derive(Default)]
pub struct KeyEventManager {
    /// Filled on first [`KeyEventManager::instance`]. `Default` cannot mint Handles.
    hardware_keyboard: Option<Handle<HardwareKeyboard>>,
}

impl KeyEventManager {
    /// The current [`KeyEventManager`], if one has been created.
    pub fn instance(app: &mut App) -> Handle<KeyEventManager> {
        let this: Handle<KeyEventManager> = app.singleton();
        if app.get(this).hardware_keyboard.is_none() {
            let hardware_keyboard = HardwareKeyboard::instance(app);
            app.get_mut(this).hardware_keyboard = Some(hardware_keyboard);
        }
        this
    }

    /// The keyboard this manager records key events on.
    pub fn hardware_keyboard(self: Handle<Self>, app: &App) -> Handle<HardwareKeyboard> {
        app.get(self)
            .hardware_keyboard
            .expect("KeyEventManager::instance must run first")
    }

    /// Dispatch a key data to the hardware keyboard and its handlers.
    ///
    /// This method is the handler to the host's `key_data` API.
    ///
    /// Returns whether a handler handled the event.
    pub fn handle_key_data(self: Handle<Self>, app: &mut App, data: KeyData) -> bool {
        // Having 0 as the physical and logical ID indicates an empty key data
        // (the only occasion either field can be 0,) transmitted to ensure
        // that the transit mode is correctly inferred. These events should be
        // ignored.
        if data.physical == 0 && data.logical == 0 {
            return false;
        }
        debug_assert!(data.physical != 0 && data.logical != 0);
        let event = KeyEventManager::event_from_data(data);
        self.hardware_keyboard(app).handle_key_event(app, &event)
    }

    fn event_from_data(key_data: KeyData) -> KeyEvent {
        let physical_key = PhysicalKeyboardKey::find_key_by_code(key_data.physical)
            .unwrap_or(PhysicalKeyboardKey::new(key_data.physical));
        let logical_key = LogicalKeyboardKey::find_key_by_key_id(key_data.logical)
            .unwrap_or(LogicalKeyboardKey::new(key_data.logical));
        let time_stamp = key_data.time_stamp;
        match key_data.event_type {
            KeyEventType::Down => {
                let mut event = KeyDownEvent::new(physical_key, logical_key, time_stamp)
                    .synthesized(key_data.synthesized)
                    .device_type(key_data.device_type);
                if let Some(character) = key_data.character {
                    event = event.character(character);
                }
                KeyEvent::Down(event)
            }
            KeyEventType::Up => {
                debug_assert!(key_data.character.is_none());
                KeyEvent::Up(
                    KeyUpEvent::new(physical_key, logical_key, time_stamp)
                        .synthesized(key_data.synthesized)
                        .device_type(key_data.device_type),
                )
            }
            KeyEventType::Repeat => {
                let mut event = KeyRepeatEvent::new(physical_key, logical_key, time_stamp)
                    .device_type(key_data.device_type);
                if let Some(character) = key_data.character {
                    event = event.character(character);
                }
                KeyEvent::Repeat(event)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::rc::Rc;
    use std::time::Duration;

    use reveal_embedder::{KeyData, KeyEventType};
    use reveal_foundation::{App, AppCell};

    use super::{
        HardwareKeyboard, KeyDownEvent, KeyEvent, KeyEventCallback, KeyEventManager,
        KeyRepeatEvent, KeyUpEvent, KeyboardLockMode,
    };
    use crate::keyboard_key::{LogicalKeyboardKey, PhysicalKeyboardKey};

    fn down(physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) -> KeyEvent {
        KeyEvent::Down(KeyDownEvent::new(physical, logical, Duration::ZERO))
    }

    fn up(physical: PhysicalKeyboardKey, logical: LogicalKeyboardKey) -> KeyEvent {
        KeyEvent::Up(KeyUpEvent::new(physical, logical, Duration::ZERO))
    }

    #[test]
    fn a_down_up_pair_updates_the_pressed_key_sets() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyboard = HardwareKeyboard::instance(&mut app);

        keyboard.handle_key_event(
            &mut app,
            &down(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A),
        );
        assert_eq!(
            keyboard.physical_keys_pressed(&app),
            HashSet::from([PhysicalKeyboardKey::KEY_A])
        );
        assert_eq!(
            keyboard.logical_keys_pressed(&app),
            HashSet::from([LogicalKeyboardKey::KEY_A])
        );
        assert!(keyboard.is_physical_key_pressed(&app, PhysicalKeyboardKey::KEY_A));
        assert!(keyboard.is_logical_key_pressed(&app, LogicalKeyboardKey::KEY_A));
        assert_eq!(
            keyboard.look_up_layout(&app, PhysicalKeyboardKey::KEY_A),
            Some(LogicalKeyboardKey::KEY_A)
        );

        keyboard.handle_key_event(
            &mut app,
            &up(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A),
        );
        assert!(keyboard.physical_keys_pressed(&app).is_empty());
        assert!(keyboard.logical_keys_pressed(&app).is_empty());
    }

    #[test]
    fn a_repeat_event_keeps_the_key_pressed_and_updates_the_logical_key() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyboard = HardwareKeyboard::instance(&mut app);

        keyboard.handle_key_event(
            &mut app,
            &down(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A),
        );
        keyboard.handle_key_event(
            &mut app,
            &KeyEvent::Repeat(
                KeyRepeatEvent::new(
                    PhysicalKeyboardKey::KEY_A,
                    LogicalKeyboardKey::KEY_Q,
                    Duration::ZERO,
                )
                .character("q"),
            ),
        );
        assert_eq!(
            keyboard.physical_keys_pressed(&app),
            HashSet::from([PhysicalKeyboardKey::KEY_A])
        );
        assert_eq!(
            keyboard.look_up_layout(&app, PhysicalKeyboardKey::KEY_A),
            Some(LogicalKeyboardKey::KEY_Q)
        );
    }

    #[test]
    fn is_shift_pressed_answers_for_either_side() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyboard = HardwareKeyboard::instance(&mut app);
        assert!(!keyboard.is_shift_pressed(&app));

        keyboard.handle_key_event(
            &mut app,
            &down(
                PhysicalKeyboardKey::SHIFT_RIGHT,
                LogicalKeyboardKey::SHIFT_RIGHT,
            ),
        );
        assert!(keyboard.is_shift_pressed(&app));
        assert!(!keyboard.is_control_pressed(&app));

        keyboard.handle_key_event(
            &mut app,
            &up(
                PhysicalKeyboardKey::SHIFT_RIGHT,
                LogicalKeyboardKey::SHIFT_RIGHT,
            ),
        );
        keyboard.handle_key_event(
            &mut app,
            &down(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            ),
        );
        assert!(keyboard.is_shift_pressed(&app));
    }

    #[test]
    fn a_lock_key_toggles_its_mode_on_every_down() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyboard = HardwareKeyboard::instance(&mut app);
        let caps_lock = down(
            PhysicalKeyboardKey::CAPS_LOCK,
            LogicalKeyboardKey::CAPS_LOCK,
        );

        keyboard.handle_key_event(&mut app, &caps_lock);
        assert_eq!(
            keyboard.lock_modes_enabled(&app),
            HashSet::from([KeyboardLockMode::CapsLock])
        );
        keyboard.handle_key_event(
            &mut app,
            &up(
                PhysicalKeyboardKey::CAPS_LOCK,
                LogicalKeyboardKey::CAPS_LOCK,
            ),
        );
        assert_eq!(
            keyboard.lock_modes_enabled(&app),
            HashSet::from([KeyboardLockMode::CapsLock])
        );
        keyboard.handle_key_event(&mut app, &caps_lock);
        assert!(keyboard.lock_modes_enabled(&app).is_empty());
    }

    #[test]
    fn handlers_receive_events_and_can_claim_them() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyboard = HardwareKeyboard::instance(&mut app);
        let seen: Rc<RefCell<Vec<LogicalKeyboardKey>>> = Rc::new(RefCell::new(Vec::new()));

        let recorded = Rc::clone(&seen);
        let observer: KeyEventCallback = Rc::new(move |_app, event: &KeyEvent| {
            recorded.borrow_mut().push(event.logical_key());
            false
        });
        let claimer: KeyEventCallback = Rc::new(|_app, _event: &KeyEvent| true);
        keyboard.add_handler(&mut app, Rc::clone(&observer));

        assert!(!keyboard.handle_key_event(
            &mut app,
            &down(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A)
        ));
        assert_eq!(*seen.borrow(), vec![LogicalKeyboardKey::KEY_A]);

        keyboard.add_handler(&mut app, Rc::clone(&claimer));
        assert!(keyboard.handle_key_event(
            &mut app,
            &up(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A)
        ));
        assert_eq!(seen.borrow().len(), 2, "every handler runs");

        keyboard.remove_handler(&mut app, &observer);
        keyboard.handle_key_event(
            &mut app,
            &down(PhysicalKeyboardKey::KEY_B, LogicalKeyboardKey::KEY_B),
        );
        assert_eq!(seen.borrow().len(), 2, "a removed handler is not called");

        keyboard.clear_state(&mut app);
        assert!(keyboard.physical_keys_pressed(&app).is_empty());
        assert!(!keyboard.handle_key_event(
            &mut app,
            &down(PhysicalKeyboardKey::KEY_C, LogicalKeyboardKey::KEY_C)
        ));
    }

    #[test]
    fn a_handler_added_during_dispatch_takes_effect_after_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyboard = HardwareKeyboard::instance(&mut app);
        let calls = Rc::new(RefCell::new(0));

        let counted = Rc::clone(&calls);
        let late: KeyEventCallback = Rc::new(move |_app, _event: &KeyEvent| {
            *counted.borrow_mut() += 1;
            false
        });
        let adder: KeyEventCallback = Rc::new(move |app: &mut App, _event: &KeyEvent| {
            let keyboard = HardwareKeyboard::instance(app);
            keyboard.add_handler(app, Rc::clone(&late));
            false
        });
        keyboard.add_handler(&mut app, adder);

        keyboard.handle_key_event(
            &mut app,
            &down(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A),
        );
        assert_eq!(*calls.borrow(), 0, "the addition waits for the dispatch");
        keyboard.handle_key_event(
            &mut app,
            &up(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A),
        );
        assert_eq!(*calls.borrow(), 1);
    }

    #[test]
    fn key_data_becomes_an_event_the_keyboard_records() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let manager = KeyEventManager::instance(&mut app);
        let keyboard = manager.hardware_keyboard(&app);

        assert!(!manager.handle_key_data(
            &mut app,
            KeyData {
                physical: 0,
                logical: 0,
                ..KeyData::default()
            }
        ));
        assert!(keyboard.physical_keys_pressed(&app).is_empty());

        manager.handle_key_data(
            &mut app,
            KeyData {
                time_stamp: Duration::from_millis(3),
                event_type: KeyEventType::Down,
                physical: PhysicalKeyboardKey::KEY_A.usb_hid_usage,
                logical: LogicalKeyboardKey::KEY_A.key_id,
                character: Some("a".to_owned()),
                ..KeyData::default()
            },
        );
        assert_eq!(
            keyboard.physical_keys_pressed(&app),
            HashSet::from([PhysicalKeyboardKey::KEY_A])
        );

        manager.handle_key_data(
            &mut app,
            KeyData {
                time_stamp: Duration::from_millis(4),
                event_type: KeyEventType::Up,
                physical: PhysicalKeyboardKey::KEY_A.usb_hid_usage,
                logical: LogicalKeyboardKey::KEY_A.key_id,
                ..KeyData::default()
            },
        );
        assert!(keyboard.physical_keys_pressed(&app).is_empty());
    }

    #[test]
    fn sync_keyboard_state_records_what_the_host_reports() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keyboard = HardwareKeyboard::instance(&mut app);
        keyboard.sync_keyboard_state(
            &mut app,
            &HashMap::from([(
                PhysicalKeyboardKey::SHIFT_LEFT.usb_hid_usage,
                LogicalKeyboardKey::SHIFT_LEFT.key_id,
            )]),
        );
        assert!(keyboard.is_shift_pressed(&app));
    }
}

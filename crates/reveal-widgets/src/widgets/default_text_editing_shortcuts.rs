//! Flutter counterpart: `widgets/default_text_editing_shortcuts.dart`.

use std::rc::Rc;

use reveal_embedder::TargetPlatform;
use reveal_foundation::{App, K_IS_WEB};
use reveal_painting::AxisDirection;
use reveal_services::SelectionChangedCause;

use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::actions::{DismissIntent, Intent, IntentRef};
use crate::widgets::focus_traversal::{NextFocusIntent, PreviousFocusIntent};
use crate::widgets::scrollable_helpers::{ScrollIncrementType, ScrollIntent};
use crate::widgets::shortcuts::{
    LockState, ShortcutActivatorRef, ShortcutMap, Shortcuts, SingleActivator,
};
use crate::widgets::text_editing_intents::{
    CopySelectionTextIntent, DeleteCharacterIntent, DeleteToLineBreakIntent,
    DeleteToNextWordBoundaryIntent, DoNothingAndStopPropagationTextIntent,
    ExpandSelectionToDocumentBoundaryIntent, ExpandSelectionToLineBreakIntent,
    ExtendSelectionByCharacterIntent, ExtendSelectionToDocumentBoundaryIntent,
    ExtendSelectionToLineBreakIntent, ExtendSelectionToNextParagraphBoundaryIntent,
    ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent,
    ExtendSelectionToNextWordBoundaryIntent,
    ExtendSelectionToNextWordBoundaryOrCaretLocationIntent,
    ExtendSelectionVerticallyToAdjacentLineIntent, ExtendSelectionVerticallyToAdjacentPageIntent,
    PasteTextIntent, RedoTextIntent, ScrollToDocumentBoundaryIntent, SelectAllTextIntent,
    TransposeCharactersIntent, UndoTextIntent,
};

/// One entry of a shortcut table: Dart's `ShortcutActivator: Intent` pair.
pub(crate) fn shortcut(
    activator: SingleActivator,
    intent: impl Intent,
) -> (ShortcutActivatorRef, IntentRef) {
    (Rc::new(activator), Rc::new(intent))
}

/// A shortcut table in the shape of Dart's map literal: a trigger key with its modifier
/// setters on the left, the intent it emits on the right.
macro_rules! shortcut_map {
    ($($key:ident $(.$modifier:ident($value:expr))* => $intent:expr),* $(,)?) => {
        vec![$($crate::widgets::default_text_editing_shortcuts::shortcut(
            $crate::widgets::shortcuts::SingleActivator::new(
                ::reveal_services::LogicalKeyboardKey::$key,
            )$(.$modifier($value))*,
            $intent,
        )),*]
    };
}

pub(crate) use shortcut_map;

/// A widget with the shortcuts used for the default text editing behavior.
///
/// This default behavior can be overridden by placing a [`Shortcuts`] widget
/// lower in the widget tree than this. See the `Action` trait for an example
/// of remapping an [`Intent`] to a custom `Action`.
///
/// The [`Shortcuts`] widget usually takes precedence over system keybindings.
/// Proceed with caution if the shortcut you wish to override is also used by
/// the system. For example, overriding `LogicalKeyboardKey::BACKSPACE` could
/// cause CJK input methods to discard more text than they should when the
/// backspace key is pressed during text composition on iOS.
///
/// See also:
///
///   * `WidgetsApp`, which creates a [`DefaultTextEditingShortcuts`].
#[derive(Debug)]
pub struct DefaultTextEditingShortcuts {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl DefaultTextEditingShortcuts {
    /// Creates a [`DefaultTextEditingShortcuts`] widget that provides the default text editing
    /// shortcuts on the current platform.
    pub fn new<K>(child: impl IntoWidget<K>) -> DefaultTextEditingShortcuts {
        DefaultTextEditingShortcuts {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `DefaultTextEditingShortcuts(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultTextEditingShortcuts {
        self.key = Some(key);
        self
    }

    /// The shortcuts of the platform the app runs on (Dart's `_shortcuts`).
    fn shortcuts(app: &App) -> ShortcutMap {
        match app.platform().target_platform() {
            TargetPlatform::Android => android_shortcuts(),
            TargetPlatform::Fuchsia => fuchsia_shortcuts(),
            TargetPlatform::IOS => ios_shortcuts(),
            TargetPlatform::Linux => linux_shortcuts(),
            TargetPlatform::MacOS => mac_shortcuts(),
            TargetPlatform::Windows => windows_shortcuts(),
        }
    }

    /// The shortcuts the platform handles itself, which the framework must hand back to it
    /// (Dart's `_getDisablingShortcut`).
    fn get_disabling_shortcut(app: &App) -> Option<ShortcutMap> {
        if K_IS_WEB {
            return match app.platform().target_platform() {
                TargetPlatform::Linux => {
                    let mut shortcuts = web_disabling_text_shortcuts();
                    for (activator, _) in linux_numpad_shortcuts() {
                        shortcuts.push((
                            activator,
                            Rc::new(DoNothingAndStopPropagationTextIntent::new()) as IntentRef,
                        ));
                    }
                    Some(shortcuts)
                }
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Windows
                | TargetPlatform::IOS
                | TargetPlatform::MacOS => Some(web_disabling_text_shortcuts()),
            };
        }
        match app.platform().target_platform() {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => None,
            TargetPlatform::IOS => Some(ios_disabling_text_shortcuts()),
            TargetPlatform::MacOS => Some(mac_disabling_text_shortcuts()),
        }
    }
}

impl StatelessWidget for DefaultTextEditingShortcuts {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut result = self.child.clone();
        if let Some(disabling_shortcut) = DefaultTextEditingShortcuts::get_disabling_shortcut(app) {
            // These shortcuts make sure of the following:
            //
            // 1. Shortcuts fired when an EditableText is focused are ignored and
            //    forwarded to the platform by the EditableText's Actions, because it
            //    maps DoNothingAndStopPropagationTextIntent to DoNothingAction.
            // 2. Shortcuts fired when no EditableText is focused will still trigger
            //    the platform shortcuts assuming DoNothingAndStopPropagationTextIntent is
            //    unhandled elsewhere.
            result = Shortcuts::new(disabling_shortcut, result)
                .debug_label("<Web Disabling Text Editing Shortcuts>")
                .into_widget();
        }
        Shortcuts::new(DefaultTextEditingShortcuts::shortcuts(app), result)
            .debug_label("<Default Text Editing Shortcuts>")
            .into_widget()
    }
}

/// These shortcuts are shared between all platforms except Apple platforms,
/// because they use different modifier keys as the line/word modifier.
fn common_shortcuts() -> ShortcutMap {
    let mut shortcuts = ShortcutMap::new();
    // Delete Shortcuts.
    for press_shift in [true, false] {
        shortcuts.append(&mut shortcut_map![
            BACKSPACE.shift(press_shift) => DeleteCharacterIntent::new(false),
            BACKSPACE.control(true).shift(press_shift) => DeleteToNextWordBoundaryIntent::new(false),
            BACKSPACE.alt(true).shift(press_shift) => DeleteToLineBreakIntent::new(false),
            DELETE.control(true).shift(press_shift) => DeleteToNextWordBoundaryIntent::new(true),
            DELETE.alt(true).shift(press_shift) => DeleteToLineBreakIntent::new(true),
        ]);
    }
    shortcuts.append(&mut shortcut_map![
        DELETE => DeleteCharacterIntent::new(true),

        // Arrow: Move selection.
        ARROW_LEFT => ExtendSelectionByCharacterIntent::new(false, true),
        ARROW_RIGHT => ExtendSelectionByCharacterIntent::new(true, true),
        ARROW_UP => ExtendSelectionVerticallyToAdjacentLineIntent::new(false, true),
        ARROW_DOWN => ExtendSelectionVerticallyToAdjacentLineIntent::new(true, true),

        // Shift + Arrow: Extend selection.
        ARROW_LEFT.shift(true) => ExtendSelectionByCharacterIntent::new(false, false),
        ARROW_RIGHT.shift(true) => ExtendSelectionByCharacterIntent::new(true, false),
        ARROW_UP.shift(true) => ExtendSelectionVerticallyToAdjacentLineIntent::new(false, false),
        ARROW_DOWN.shift(true) => ExtendSelectionVerticallyToAdjacentLineIntent::new(true, false),

        ARROW_LEFT.alt(true) => ExtendSelectionToLineBreakIntent::new(false, true),
        ARROW_RIGHT.alt(true) => ExtendSelectionToLineBreakIntent::new(true, true),
        ARROW_UP.alt(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, true),
        ARROW_DOWN.alt(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, true),

        ARROW_LEFT.shift(true).alt(true) => ExtendSelectionToLineBreakIntent::new(false, false),
        ARROW_RIGHT.shift(true).alt(true) => ExtendSelectionToLineBreakIntent::new(true, false),
        ARROW_UP.shift(true).alt(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, false),
        ARROW_DOWN.shift(true).alt(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, false),

        ARROW_LEFT.control(true) => ExtendSelectionToNextWordBoundaryIntent::new(false, true),
        ARROW_RIGHT.control(true) => ExtendSelectionToNextWordBoundaryIntent::new(true, true),

        ARROW_LEFT.shift(true).control(true) =>
            ExtendSelectionToNextWordBoundaryIntent::new(false, false),
        ARROW_RIGHT.shift(true).control(true) =>
            ExtendSelectionToNextWordBoundaryIntent::new(true, false),

        ARROW_UP.shift(true).control(true) =>
            ExtendSelectionToNextParagraphBoundaryIntent::new(false, false),
        ARROW_DOWN.shift(true).control(true) =>
            ExtendSelectionToNextParagraphBoundaryIntent::new(true, false),

        // Page Up / Down: Move selection by page.
        PAGE_UP => ExtendSelectionVerticallyToAdjacentPageIntent::new(false, true),
        PAGE_DOWN => ExtendSelectionVerticallyToAdjacentPageIntent::new(true, true),

        // Shift + Page Up / Down: Extend selection by page.
        PAGE_UP.shift(true) => ExtendSelectionVerticallyToAdjacentPageIntent::new(false, false),
        PAGE_DOWN.shift(true) => ExtendSelectionVerticallyToAdjacentPageIntent::new(true, false),
    ]);
    shortcuts
}

fn clipboard_shortcuts() -> ShortcutMap {
    shortcut_map![
        // Xerox/Apple: ^X ^C ^V
        // -> Standard on Windows
        // -> Standard on Linux
        // -> Standard on Mac OS X (with Command as modifier)
        KEY_X.control(true) => CopySelectionTextIntent::cut(SelectionChangedCause::Keyboard),
        KEY_C.control(true) => CopySelectionTextIntent::COPY,
        KEY_V.control(true) => PasteTextIntent::new(SelectionChangedCause::Keyboard),

        // IBM CUA guidelines: Shift-Del Ctrl-Ins Shift-Ins
        // -> Standard on Windows
        // -> Standard on Linux (traditionally mapped to the Selection buffer rather than the
        //                       Clipboard, but the distinction is often no longer present with
        //                       modern toolkits)
        // -> Not standard on Mac OS X
        DELETE.shift(true) => CopySelectionTextIntent::cut(SelectionChangedCause::Keyboard),
        INSERT.control(true) => CopySelectionTextIntent::COPY,
        INSERT.shift(true) => PasteTextIntent::new(SelectionChangedCause::Keyboard),

        KEY_A.control(true) => SelectAllTextIntent::new(SelectionChangedCause::Keyboard),
        KEY_Z.control(true) => UndoTextIntent::new(SelectionChangedCause::Keyboard),
        KEY_Z.shift(true).control(true) => RedoTextIntent::new(SelectionChangedCause::Keyboard),
        // These keys should go to the IME when a field is focused, not to other
        // Shortcuts.
        SPACE => DoNothingAndStopPropagationTextIntent::new(),
        ENTER => DoNothingAndStopPropagationTextIntent::new(),
    ]
}

/// The following key combinations have no effect on text editing on this
/// platform:
///   * Meta + X
///   * Meta + C
///   * Meta + V
///   * Meta + A
///   * Meta + shift? + Z
///   * Meta + shift? + arrow down
///   * Meta + shift? + arrow left
///   * Meta + shift? + arrow right
///   * Meta + shift? + arrow up
///   * Meta + shift? + delete
///   * Meta + shift? + backspace
fn android_shortcuts() -> ShortcutMap {
    let mut shortcuts = common_shortcuts();
    shortcuts.append(&mut clipboard_shortcuts());
    shortcuts.append(&mut shortcut_map![
        HOME => ExtendSelectionToLineBreakIntent::new(false, true).continues_at_wrap(true),
        END => ExtendSelectionToLineBreakIntent::new(true, true).continues_at_wrap(true),
        HOME.shift(true) =>
            ExtendSelectionToLineBreakIntent::new(false, false).continues_at_wrap(true),
        END.shift(true) =>
            ExtendSelectionToLineBreakIntent::new(true, false).continues_at_wrap(true),
        HOME.control(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, true),
        END.control(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, true),
        HOME.shift(true).control(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, false),
        END.shift(true).control(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, false),
    ]);
    shortcuts
}

fn fuchsia_shortcuts() -> ShortcutMap {
    android_shortcuts()
}

fn linux_numpad_shortcuts() -> ShortcutMap {
    shortcut_map![
        // When numLock is on, numpad keys shortcuts require shift to be pressed too.
        NUMPAD6.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionByCharacterIntent::new(true, false),
        NUMPAD4.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionByCharacterIntent::new(false, false),
        NUMPAD8.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(false, false),
        NUMPAD2.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(true, false),

        NUMPAD6.shift(true).control(true).num_lock(LockState::Locked) =>
            ExtendSelectionToNextWordBoundaryIntent::new(true, false),
        NUMPAD4.shift(true).control(true).num_lock(LockState::Locked) =>
            ExtendSelectionToNextWordBoundaryIntent::new(false, false),
        NUMPAD8.shift(true).control(true).num_lock(LockState::Locked) =>
            ExtendSelectionToNextParagraphBoundaryIntent::new(false, false),
        NUMPAD2.shift(true).control(true).num_lock(LockState::Locked) =>
            ExtendSelectionToNextParagraphBoundaryIntent::new(true, false),

        NUMPAD9.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionVerticallyToAdjacentPageIntent::new(false, false),
        NUMPAD3.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionVerticallyToAdjacentPageIntent::new(true, false),

        NUMPAD7.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(false, false),
        NUMPAD1.shift(true).num_lock(LockState::Locked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(true, false),

        NUMPAD_DECIMAL.shift(true).num_lock(LockState::Locked) =>
            DeleteCharacterIntent::new(true),
        NUMPAD_DECIMAL.shift(true).control(true).num_lock(LockState::Locked) =>
            DeleteToNextWordBoundaryIntent::new(true),

        // When numLock is off, numpad keys shortcuts require shift not to be pressed.
        NUMPAD6.num_lock(LockState::Unlocked) =>
            ExtendSelectionByCharacterIntent::new(true, true),
        NUMPAD4.num_lock(LockState::Unlocked) =>
            ExtendSelectionByCharacterIntent::new(false, true),
        NUMPAD8.num_lock(LockState::Unlocked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(false, true),
        NUMPAD2.num_lock(LockState::Unlocked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(true, true),

        NUMPAD6.control(true).num_lock(LockState::Unlocked) =>
            ExtendSelectionToNextWordBoundaryIntent::new(true, true),
        NUMPAD4.control(true).num_lock(LockState::Unlocked) =>
            ExtendSelectionToNextWordBoundaryIntent::new(false, true),
        NUMPAD8.control(true).num_lock(LockState::Unlocked) =>
            ExtendSelectionToNextParagraphBoundaryIntent::new(false, true),
        NUMPAD2.control(true).num_lock(LockState::Unlocked) =>
            ExtendSelectionToNextParagraphBoundaryIntent::new(true, true),

        NUMPAD9.num_lock(LockState::Unlocked) =>
            ExtendSelectionVerticallyToAdjacentPageIntent::new(false, true),
        NUMPAD3.num_lock(LockState::Unlocked) =>
            ExtendSelectionVerticallyToAdjacentPageIntent::new(true, true),

        NUMPAD7.num_lock(LockState::Unlocked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(false, true),
        NUMPAD1.num_lock(LockState::Unlocked) =>
            ExtendSelectionVerticallyToAdjacentLineIntent::new(true, true),

        NUMPAD_DECIMAL.num_lock(LockState::Unlocked) => DeleteCharacterIntent::new(true),
        NUMPAD_DECIMAL.control(true).num_lock(LockState::Unlocked) =>
            DeleteToNextWordBoundaryIntent::new(true),
    ]
}

/// The following key combinations have no effect on text editing on this
/// platform:
///   * Control + shift? + end
///   * Control + shift? + home
///   * Meta + X
///   * Meta + C
///   * Meta + V
///   * Meta + A
///   * Meta + shift? + Z
///   * Meta + shift? + arrow down
///   * Meta + shift? + arrow left
///   * Meta + shift? + arrow right
///   * Meta + shift? + arrow up
///   * Meta + shift? + delete
///   * Meta + shift? + backspace
fn linux_shortcuts() -> ShortcutMap {
    let mut shortcuts = common_shortcuts();
    shortcuts.append(&mut clipboard_shortcuts());
    shortcuts.append(&mut linux_numpad_shortcuts());
    shortcuts.append(&mut shortcut_map![
        HOME => ExtendSelectionToLineBreakIntent::new(false, true),
        END => ExtendSelectionToLineBreakIntent::new(true, true),
        HOME.shift(true) => ExtendSelectionToLineBreakIntent::new(false, false),
        END.shift(true) => ExtendSelectionToLineBreakIntent::new(true, false),
        HOME.control(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, true),
        END.control(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, true),
        HOME.shift(true).control(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, false),
        END.shift(true).control(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, false),
    ]);
    shortcuts
}

/// macOS document shortcuts: <https://support.apple.com/en-us/HT201236>.
/// The macOS shortcuts uses different word/line modifiers than most other
/// platforms.
///
/// The following key combinations have no effect on text editing on this
/// platform:
///   * End
///   * Home
///   * Control + shift? + end
///   * Control + shift? + home
///   * Control + shift? + Z
fn mac_shortcuts() -> ShortcutMap {
    let mut shortcuts = ShortcutMap::new();
    for press_shift in [true, false] {
        shortcuts.append(&mut shortcut_map![
            BACKSPACE.shift(press_shift) => DeleteCharacterIntent::new(false),
            BACKSPACE.alt(true).shift(press_shift) => DeleteToNextWordBoundaryIntent::new(false),
            BACKSPACE.meta(true).shift(press_shift) => DeleteToLineBreakIntent::new(false),
            DELETE.shift(press_shift) => DeleteCharacterIntent::new(true),
            DELETE.alt(true).shift(press_shift) => DeleteToNextWordBoundaryIntent::new(true),
            DELETE.meta(true).shift(press_shift) => DeleteToLineBreakIntent::new(true),
        ]);
    }
    shortcuts.append(&mut shortcut_map![
        ARROW_LEFT => ExtendSelectionByCharacterIntent::new(false, true),
        ARROW_RIGHT => ExtendSelectionByCharacterIntent::new(true, true),
        ARROW_UP => ExtendSelectionVerticallyToAdjacentLineIntent::new(false, true),
        ARROW_DOWN => ExtendSelectionVerticallyToAdjacentLineIntent::new(true, true),

        // Shift + Arrow: Extend selection.
        ARROW_LEFT.shift(true) => ExtendSelectionByCharacterIntent::new(false, false),
        ARROW_RIGHT.shift(true) => ExtendSelectionByCharacterIntent::new(true, false),
        ARROW_UP.shift(true) => ExtendSelectionVerticallyToAdjacentLineIntent::new(false, false),
        ARROW_DOWN.shift(true) => ExtendSelectionVerticallyToAdjacentLineIntent::new(true, false),

        ARROW_LEFT.alt(true) => ExtendSelectionToNextWordBoundaryIntent::new(false, true),
        ARROW_RIGHT.alt(true) => ExtendSelectionToNextWordBoundaryIntent::new(true, true),
        ARROW_UP.alt(true) => ExtendSelectionToLineBreakIntent::new(false, true),
        ARROW_DOWN.alt(true) => ExtendSelectionToLineBreakIntent::new(true, true),

        ARROW_LEFT.shift(true).alt(true) =>
            ExtendSelectionToNextWordBoundaryOrCaretLocationIntent::new(false),
        ARROW_RIGHT.shift(true).alt(true) =>
            ExtendSelectionToNextWordBoundaryOrCaretLocationIntent::new(true),
        ARROW_UP.shift(true).alt(true) =>
            ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent::new(false),
        ARROW_DOWN.shift(true).alt(true) =>
            ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent::new(true),

        ARROW_LEFT.meta(true) => ExtendSelectionToLineBreakIntent::new(false, true),
        ARROW_RIGHT.meta(true) => ExtendSelectionToLineBreakIntent::new(true, true),
        ARROW_UP.meta(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, true),
        ARROW_DOWN.meta(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, true),

        ARROW_LEFT.shift(true).meta(true) => ExpandSelectionToLineBreakIntent::new(false),
        ARROW_RIGHT.shift(true).meta(true) => ExpandSelectionToLineBreakIntent::new(true),
        ARROW_UP.shift(true).meta(true) => ExpandSelectionToDocumentBoundaryIntent::new(false),
        ARROW_DOWN.shift(true).meta(true) => ExpandSelectionToDocumentBoundaryIntent::new(true),

        KEY_T.control(true) => TransposeCharactersIntent::new(),

        HOME => ScrollToDocumentBoundaryIntent::new(false),
        END => ScrollToDocumentBoundaryIntent::new(true),
        HOME.shift(true) => ExpandSelectionToDocumentBoundaryIntent::new(false),
        END.shift(true) => ExpandSelectionToDocumentBoundaryIntent::new(true),

        PAGE_UP => ScrollIntent::new(AxisDirection::Up).r#type(ScrollIncrementType::Page),
        PAGE_DOWN => ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Page),
        PAGE_UP.shift(true) => ExtendSelectionVerticallyToAdjacentPageIntent::new(false, false),
        PAGE_DOWN.shift(true) => ExtendSelectionVerticallyToAdjacentPageIntent::new(true, false),

        KEY_X.meta(true) => CopySelectionTextIntent::cut(SelectionChangedCause::Keyboard),
        KEY_C.meta(true) => CopySelectionTextIntent::COPY,
        KEY_V.meta(true) => PasteTextIntent::new(SelectionChangedCause::Keyboard),
        KEY_A.meta(true) => SelectAllTextIntent::new(SelectionChangedCause::Keyboard),
        KEY_Z.meta(true) => UndoTextIntent::new(SelectionChangedCause::Keyboard),
        KEY_Z.shift(true).meta(true) => RedoTextIntent::new(SelectionChangedCause::Keyboard),
        KEY_E.control(true) => ExtendSelectionToLineBreakIntent::new(true, true),
        KEY_A.control(true) => ExtendSelectionToLineBreakIntent::new(false, true),
        KEY_F.control(true) => ExtendSelectionByCharacterIntent::new(true, true),
        KEY_B.control(true) => ExtendSelectionByCharacterIntent::new(false, true),
        KEY_N.control(true) => ExtendSelectionVerticallyToAdjacentLineIntent::new(true, true),
        KEY_P.control(true) => ExtendSelectionVerticallyToAdjacentLineIntent::new(false, true),
        // These keys should go to the IME when a field is focused, not to other
        // Shortcuts.
        SPACE => DoNothingAndStopPropagationTextIntent::new(),
        ENTER => DoNothingAndStopPropagationTextIntent::new(),
    ]);
    shortcuts
}

/// There is no complete documentation of iOS shortcuts: use macOS ones.
fn ios_shortcuts() -> ShortcutMap {
    mac_shortcuts()
}

/// The following key combinations have no effect on text editing on this
/// platform:
///   * Meta + X
///   * Meta + C
///   * Meta + V
///   * Meta + A
///   * Meta + shift? + arrow down
///   * Meta + shift? + arrow left
///   * Meta + shift? + arrow right
///   * Meta + shift? + arrow up
///   * Meta + delete
///   * Meta + backspace
fn windows_shortcuts() -> ShortcutMap {
    let mut shortcuts = common_shortcuts();
    shortcuts.append(&mut clipboard_shortcuts());
    shortcuts.append(&mut shortcut_map![
        PAGE_UP => ExtendSelectionVerticallyToAdjacentPageIntent::new(false, true),
        PAGE_DOWN => ExtendSelectionVerticallyToAdjacentPageIntent::new(true, true),
        HOME => ExtendSelectionToLineBreakIntent::new(false, true).continues_at_wrap(true),
        END => ExtendSelectionToLineBreakIntent::new(true, true).continues_at_wrap(true),
        HOME.shift(true) =>
            ExtendSelectionToLineBreakIntent::new(false, false).continues_at_wrap(true),
        END.shift(true) =>
            ExtendSelectionToLineBreakIntent::new(true, false).continues_at_wrap(true),
        HOME.control(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, true),
        END.control(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, true),
        HOME.shift(true).control(true) => ExtendSelectionToDocumentBoundaryIntent::new(false, false),
        END.shift(true).control(true) => ExtendSelectionToDocumentBoundaryIntent::new(true, false),
    ]);
    shortcuts
}

/// Web handles its text selection natively and doesn't use any of these
/// shortcuts in Flutter.
fn web_disabling_text_shortcuts() -> ShortcutMap {
    let mut shortcuts = ShortcutMap::new();
    for press_shift in [true, false] {
        shortcuts.append(&mut shortcut_map![
            BACKSPACE.shift(press_shift) => DoNothingAndStopPropagationTextIntent::new(),
            DELETE.shift(press_shift) => DoNothingAndStopPropagationTextIntent::new(),
            BACKSPACE.alt(true).shift(press_shift) => DoNothingAndStopPropagationTextIntent::new(),
            DELETE.alt(true).shift(press_shift) => DoNothingAndStopPropagationTextIntent::new(),
            BACKSPACE.control(true).shift(press_shift) =>
                DoNothingAndStopPropagationTextIntent::new(),
            DELETE.control(true).shift(press_shift) => DoNothingAndStopPropagationTextIntent::new(),
            BACKSPACE.meta(true).shift(press_shift) => DoNothingAndStopPropagationTextIntent::new(),
            DELETE.meta(true).shift(press_shift) => DoNothingAndStopPropagationTextIntent::new(),
        ]);
    }
    shortcuts.append(&mut common_disabling_text_shortcuts());
    for (activator, _) in clipboard_shortcuts() {
        shortcuts.push((
            activator,
            Rc::new(DoNothingAndStopPropagationTextIntent::new()) as IntentRef,
        ));
    }
    shortcuts.append(&mut shortcut_map![
        KEY_X.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        KEY_C.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        KEY_V.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        KEY_A.control(true) => DoNothingAndStopPropagationTextIntent::new(),
        KEY_A.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
    ]);
    shortcuts
}

fn common_disabling_text_shortcuts() -> ShortcutMap {
    shortcut_map![
        ARROW_DOWN.alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT.alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT.alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_UP.alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_DOWN.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_UP.meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_DOWN => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_UP => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT.control(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT.control(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT.shift(true).control(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT.shift(true).control(true) => DoNothingAndStopPropagationTextIntent::new(),
        SPACE => DoNothingAndStopPropagationTextIntent::new(),
        ENTER => DoNothingAndStopPropagationTextIntent::new(),
    ]
}

fn mac_disabling_text_shortcuts() -> ShortcutMap {
    let mut shortcuts = common_disabling_text_shortcuts();
    shortcuts.append(&mut ios_disabling_text_shortcuts());
    shortcuts.append(&mut shortcut_map![
        ESCAPE => DoNothingAndStopPropagationTextIntent::new(),
        TAB => DoNothingAndStopPropagationTextIntent::new(),
        TAB.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_DOWN.shift(true).alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_UP.shift(true).alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT.shift(true).alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT.shift(true).alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_LEFT.shift(true).meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        ARROW_RIGHT.shift(true).meta(true) => DoNothingAndStopPropagationTextIntent::new(),
        PAGE_UP => DoNothingAndStopPropagationTextIntent::new(),
        PAGE_DOWN => DoNothingAndStopPropagationTextIntent::new(),
        END => DoNothingAndStopPropagationTextIntent::new(),
        HOME => DoNothingAndStopPropagationTextIntent::new(),
        PAGE_UP.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        PAGE_DOWN.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        END.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        HOME.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        END.control(true) => DoNothingAndStopPropagationTextIntent::new(),
        HOME.control(true) => DoNothingAndStopPropagationTextIntent::new(),
    ]);
    shortcuts
}

/// Hand backspace/delete events that do not depend on text layout (delete
/// character and delete to the next word) back to the IME to allow it to
/// update composing text properly.
fn ios_disabling_text_shortcuts() -> ShortcutMap {
    shortcut_map![
        BACKSPACE => DoNothingAndStopPropagationTextIntent::new(),
        BACKSPACE.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        DELETE => DoNothingAndStopPropagationTextIntent::new(),
        DELETE.shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        BACKSPACE.alt(true).shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        BACKSPACE.alt(true) => DoNothingAndStopPropagationTextIntent::new(),
        DELETE.alt(true).shift(true) => DoNothingAndStopPropagationTextIntent::new(),
        DELETE.alt(true) => DoNothingAndStopPropagationTextIntent::new(),
    ]
}

/// Maps the selector from `NSStandardKeyBindingResponding` to the [`Intent`] if the
/// selector is recognized.
pub fn intent_for_macos_selector(selector_name: &str) -> Option<IntentRef> {
    let intent: IntentRef = match selector_name {
        "deleteBackward:" => Rc::new(DeleteCharacterIntent::new(false)),
        "deleteWordBackward:" => Rc::new(DeleteToNextWordBoundaryIntent::new(false)),
        "deleteToBeginningOfLine:" => Rc::new(DeleteToLineBreakIntent::new(false)),
        "deleteForward:" => Rc::new(DeleteCharacterIntent::new(true)),
        "deleteWordForward:" => Rc::new(DeleteToNextWordBoundaryIntent::new(true)),
        "deleteToEndOfLine:" => Rc::new(DeleteToLineBreakIntent::new(true)),

        "moveLeft:" => Rc::new(ExtendSelectionByCharacterIntent::new(false, true)),
        "moveRight:" => Rc::new(ExtendSelectionByCharacterIntent::new(true, true)),
        "moveForward:" => Rc::new(ExtendSelectionByCharacterIntent::new(true, true)),
        "moveBackward:" => Rc::new(ExtendSelectionByCharacterIntent::new(false, true)),

        "moveUp:" => Rc::new(ExtendSelectionVerticallyToAdjacentLineIntent::new(
            false, true,
        )),
        "moveDown:" => Rc::new(ExtendSelectionVerticallyToAdjacentLineIntent::new(
            true, true,
        )),

        "moveLeftAndModifySelection:" => {
            Rc::new(ExtendSelectionByCharacterIntent::new(false, false))
        }
        "moveRightAndModifySelection:" => {
            Rc::new(ExtendSelectionByCharacterIntent::new(true, false))
        }
        "moveUpAndModifySelection:" => Rc::new(ExtendSelectionVerticallyToAdjacentLineIntent::new(
            false, false,
        )),
        "moveDownAndModifySelection:" => Rc::new(
            ExtendSelectionVerticallyToAdjacentLineIntent::new(true, false),
        ),

        "moveWordLeft:" => Rc::new(ExtendSelectionToNextWordBoundaryIntent::new(false, true)),
        "moveWordRight:" => Rc::new(ExtendSelectionToNextWordBoundaryIntent::new(true, true)),
        "moveToBeginningOfParagraph:" => {
            Rc::new(ExtendSelectionToLineBreakIntent::new(false, true))
        }
        "moveToEndOfParagraph:" => Rc::new(ExtendSelectionToLineBreakIntent::new(true, true)),

        "moveWordLeftAndModifySelection:" => Rc::new(
            ExtendSelectionToNextWordBoundaryOrCaretLocationIntent::new(false),
        ),
        "moveWordRightAndModifySelection:" => Rc::new(
            ExtendSelectionToNextWordBoundaryOrCaretLocationIntent::new(true),
        ),
        "moveParagraphBackwardAndModifySelection:" => {
            Rc::new(ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent::new(false))
        }
        "moveParagraphForwardAndModifySelection:" => {
            Rc::new(ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent::new(true))
        }

        "moveToLeftEndOfLine:" => Rc::new(ExtendSelectionToLineBreakIntent::new(false, true)),
        "moveToRightEndOfLine:" => Rc::new(ExtendSelectionToLineBreakIntent::new(true, true)),
        "moveToBeginningOfDocument:" => {
            Rc::new(ExtendSelectionToDocumentBoundaryIntent::new(false, true))
        }
        "moveToEndOfDocument:" => Rc::new(ExtendSelectionToDocumentBoundaryIntent::new(true, true)),

        "moveToLeftEndOfLineAndModifySelection:" => {
            Rc::new(ExpandSelectionToLineBreakIntent::new(false))
        }
        "moveToRightEndOfLineAndModifySelection:" => {
            Rc::new(ExpandSelectionToLineBreakIntent::new(true))
        }
        "moveToBeginningOfDocumentAndModifySelection:" => {
            Rc::new(ExpandSelectionToDocumentBoundaryIntent::new(false))
        }
        "moveToEndOfDocumentAndModifySelection:" => {
            Rc::new(ExpandSelectionToDocumentBoundaryIntent::new(true))
        }

        "transpose:" => Rc::new(TransposeCharactersIntent::new()),

        "scrollToBeginningOfDocument:" => Rc::new(ScrollToDocumentBoundaryIntent::new(false)),
        "scrollToEndOfDocument:" => Rc::new(ScrollToDocumentBoundaryIntent::new(true)),

        "scrollPageUp:" => {
            Rc::new(ScrollIntent::new(AxisDirection::Up).r#type(ScrollIncrementType::Page))
        }
        "scrollPageDown:" => {
            Rc::new(ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Page))
        }
        "pageUpAndModifySelection:" => Rc::new(ExtendSelectionVerticallyToAdjacentPageIntent::new(
            false, false,
        )),
        "pageDownAndModifySelection:" => Rc::new(
            ExtendSelectionVerticallyToAdjacentPageIntent::new(true, false),
        ),

        // Escape key when there's no IME selection popup.
        "cancelOperation:" => Rc::new(DismissIntent::new()),
        // Tab when there's no IME selection.
        "insertTab:" => Rc::new(NextFocusIntent::new()),
        "insertBacktab:" => Rc::new(PreviousFocusIntent::new()),

        _ => return None,
    };
    Some(intent)
}

#[cfg(test)]
mod tests {
    use reveal_embedder::{Platform, PlatformRef, ViewId, ViewRef};
    use reveal_foundation::AppCell;

    use super::*;
    use crate::binding::WidgetsBinding;
    use crate::framework::{AnyElement, downcast_widget};
    use crate::widgets::basic::SizedBox;
    use crate::widgets::focus_manager::tests::{app_with_view, mount};

    /// A platform with no views, for the tables that only read the target platform.
    struct PlatformOf(TargetPlatform);

    impl Platform for PlatformOf {
        fn target_platform(&self) -> TargetPlatform {
            self.0
        }

        fn request_frame(&self) {}

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }
    }

    fn app_of(platform: TargetPlatform) -> Rc<AppCell> {
        AppCell::with_platform(Rc::new(PlatformOf(platform)) as PlatformRef)
    }

    /// How the copy shortcut of a platform's table is described.
    fn copy_shortcut(shortcuts: &ShortcutMap) -> String {
        let (activator, _) = shortcuts
            .iter()
            .find(|(_, intent)| {
                intent
                    .as_any()
                    .downcast_ref::<CopySelectionTextIntent>()
                    .is_some_and(|copy| !copy.collapse_selection())
            })
            .expect("a copy shortcut");
        activator.debug_describe_keys()
    }

    /// The debug label and map of every [`Shortcuts`] widget of the mounted tree, outermost
    /// first.
    fn mounted_shortcuts(app: &mut App) -> Vec<(String, ShortcutMap)> {
        let root = WidgetsBinding::instance(app)
            .root_element(app)
            .expect("a mounted tree");
        let mut all: Vec<AnyElement> = vec![root];
        let mut visited = 0;
        while visited < all.len() {
            let element = all[visited];
            element.visit_children(app, &mut |child| all.push(child));
            visited += 1;
        }
        let mut found = Vec::new();
        for element in all {
            let app: &App = app;
            if let Some(widget) = downcast_widget::<Shortcuts>(&**element.widget(app)) {
                let label = widget.debug_label.clone().unwrap_or_default();
                found.push((label, widget.get_shortcuts(app)));
            }
        }
        found
    }

    #[test]
    fn every_platform_table_binds_its_own_copy_shortcut_to_the_copy_intent() {
        assert_eq!(copy_shortcut(&mac_shortcuts()), "Meta + Key C");
        assert_eq!(copy_shortcut(&ios_shortcuts()), "Meta + Key C");
        assert_eq!(copy_shortcut(&android_shortcuts()), "Control + Key C");
        assert_eq!(copy_shortcut(&fuchsia_shortcuts()), "Control + Key C");
        assert_eq!(copy_shortcut(&linux_shortcuts()), "Control + Key C");
        assert_eq!(copy_shortcut(&windows_shortcuts()), "Control + Key C");
    }

    #[test]
    fn the_cut_intent_collapses_the_selection_and_the_copy_intent_does_not() {
        let cut = CopySelectionTextIntent::cut(SelectionChangedCause::Keyboard);
        assert!(cut.collapse_selection());
        assert_eq!(cut.cause(), SelectionChangedCause::Keyboard);
        assert!(!CopySelectionTextIntent::COPY.collapse_selection());
        assert_eq!(
            CopySelectionTextIntent::COPY.cause(),
            SelectionChangedCause::Keyboard
        );
    }

    #[test]
    fn an_apple_platform_hands_the_keys_it_handles_itself_back_to_the_ime() {
        let mac_cell = app_of(TargetPlatform::MacOS);
        let mac = mac_cell.borrow();
        let disabling = DefaultTextEditingShortcuts::get_disabling_shortcut(&mac)
            .expect("macOS disables the shortcuts the platform handles");
        assert!(disabling.iter().all(|(_, intent)| {
            intent
                .as_any()
                .is::<DoNothingAndStopPropagationTextIntent>()
        }));
        let linux_cell = app_of(TargetPlatform::Linux);
        let linux = linux_cell.borrow();
        assert!(
            DefaultTextEditingShortcuts::get_disabling_shortcut(&linux).is_none(),
            "a non-Apple, non-web platform disables nothing"
        );
    }

    #[test]
    fn the_widget_wraps_its_child_in_the_platform_table_and_the_disabling_table() {
        let cell = app_with_view();
        mount(
            &cell,
            DefaultTextEditingShortcuts::new(SizedBox::shrink()).into_widget(),
        );
        let mut app = cell.borrow_mut();
        let shortcuts = mounted_shortcuts(&mut app);
        let labels: Vec<&str> = shortcuts.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "<Default Text Editing Shortcuts>",
                "<Web Disabling Text Editing Shortcuts>",
            ],
            "the disabling table is nested inside, so it is found first"
        );
        assert_eq!(copy_shortcut(&shortcuts[0].1), "Meta + Key C");
    }
}

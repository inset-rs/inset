//! A view's text input, as the framework drives it through `TextInputHost`: the input it
//! opened, the editing value it last set, and where the system's IME shows its candidates.
//! winit's IME events (`ime.rs`) change that value, and a key the framework did not handle
//! reaches the active input, which inserts its text or acts on Enter: the key half of
//! Flutter's macOS `FlutterTextInputPlugin.handleKeyEvent`.
//!
//! winit reports `Ime::Commit` only for text an input method composed (`hasMarkedText`); plain
//! typing arrives as `KeyEvent::text`, which the text input commits itself (`type_key`).

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Weak};

use inset_embedder::{
    Matrix4, Offset, Rect, Size, TextEditingValue, TextInputAction, TextInputConfiguration,
    TextInputHost, TextInputType, transform3,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{Ime, KeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{
    ImeCapabilities, ImeEnableRequest, ImeHint, ImePurpose, ImeRequest, ImeRequestData, Window,
};

use crate::ime::{ImeOutcome, apply_commit, apply_ime};
use crate::keys;
use crate::os::{NativeMenus, ThisOs};

/// One view's text input: winit's IME on the view's window, and the editing value both sides
/// change.
pub(crate) struct TextInput {
    /// The view's window, held weakly: the text input of a view with no window, before its
    /// window opens or once it closed, does nothing.
    window: RefCell<Option<Weak<dyn Window>>>,
    /// Set from `TextInputHost::start` to `TextInputHost::stop`; an unhandled key types into it.
    active: Cell<Option<ActiveTextInput>>,
    /// The editing value as the framework last set it, and as the IME's events changed it since.
    editing_value: RefCell<TextEditingValue>,
    /// Where the input is in the view, for placing the IME's candidates.
    transform: RefCell<Option<Matrix4>>,
}

impl TextInput {
    /// The text input of a view with no window yet.
    pub(crate) fn new() -> TextInput {
        TextInput {
            window: RefCell::new(None),
            active: Cell::new(None),
            editing_value: RefCell::new(TextEditingValue::EMPTY),
            transform: RefCell::new(None),
        }
    }

    /// The view's window, while it is open.
    fn window(&self) -> Option<Arc<dyn Window>> {
        self.window.borrow().as_ref()?.upgrade()
    }

    /// The view's window from now on.
    pub(crate) fn attach(&self, window: &Arc<dyn Window>) {
        *self.window.borrow_mut() = Some(Arc::downgrade(window));
    }

    /// What an IME event makes of the editing value; a new value is the last one from now on.
    pub(crate) fn take_ime(&self, ime: &Ime) -> ImeOutcome {
        let outcome = apply_ime(&self.editing_value.borrow(), ime);
        if let ImeOutcome::Value(value) = &outcome {
            *self.editing_value.borrow_mut() = value.clone();
        }
        outcome
    }

    /// What a key the framework did not handle does to the active input, with `modifiers` held:
    /// its text goes in as an input method's commit would; nothing while no input is active.
    pub(crate) fn type_key(&self, event: &KeyEvent, modifiers: ModifiersState) -> Typed {
        let Some(active) = self.active.get() else {
            return Typed::Nothing;
        };
        match active.key_effect(&event.logical_key, event.text.as_deref(), modifiers) {
            TextInputKeyEffect::None => Typed::Nothing,
            TextInputKeyEffect::Insert(text) => Typed::Edited(self.commit(&text)),
            TextInputKeyEffect::Enter { insert, action } => Typed::Entered {
                edited: insert.map(|text| self.commit(&text)),
                action,
            },
        }
    }

    /// The editing value with `text` committed in place of what was composing or selected,
    /// which is the last one from now on.
    fn commit(&self, text: &str) -> TextEditingValue {
        let value = apply_commit(&self.editing_value.borrow(), text);
        *self.editing_value.borrow_mut() = value.clone();
        value
    }

    /// Shows the IME's candidates at `rect`, in the input's coordinates, which its transform
    /// takes to the view's.
    fn set_ime_area(&self, rect: Rect) {
        let origin = match *self.transform.borrow() {
            Some(transform) => transform3(transform, Offset::new(rect.left, rect.top)),
            None => Offset::new(rect.left, rect.top),
        };
        let Some(window) = self.window() else {
            return;
        };
        let scale = window.scale_factor();
        let _ = window.request_ime_update(ImeRequest::Update(
            ImeRequestData::default().with_cursor_area(
                PhysicalPosition::new(origin.dx() * scale, origin.dy() * scale).into(),
                PhysicalSize::new(rect.width() * scale, rect.height() * scale).into(),
            ),
        ));
    }
}

impl TextInputHost for TextInput {
    /// While the system's menus send Edit's cut, copy and paste, as macOS's menu bar does while it
    /// holds those items, which a key the framework leaves reaches (`menus/`).
    fn provides_edit_commands(&self) -> bool {
        ThisOs::provides_edit_commands()
    }

    /// Turns the window's input method on for this input, its purpose a password's where the text
    /// is obscured, and its candidates placed where the framework says the caret is from now on;
    /// an input method on for another input is turned off first, as winit asks.
    fn start(&self, configuration: &TextInputConfiguration) {
        self.active.set(Some(ActiveTextInput::new(configuration)));
        let Some(window) = self.window() else {
            return;
        };
        let purpose = if configuration.obscure_text {
            ImePurpose::Password
        } else {
            ImePurpose::Normal
        };
        let _ = window.request_ime_update(ImeRequest::Disable);
        let _ = window.request_ime_update(ImeRequest::Enable(enable_request(purpose)));
    }

    fn stop(&self) {
        self.active.set(None);
        if let Some(window) = self.window() {
            let _ = window.request_ime_update(ImeRequest::Disable);
        }
    }

    fn set_editing_state(&self, value: &TextEditingValue) {
        *self.editing_value.borrow_mut() = value.clone();
    }

    fn set_composing_rect(&self, rect: Rect) {
        self.set_ime_area(rect);
    }

    fn set_caret_rect(&self, rect: Rect) {
        self.set_ime_area(rect);
    }

    fn set_client_geometry(&self, _size: Size, transform: &Matrix4) {
        *self.transform.borrow_mut() = Some(*transform);
    }
}

/// The request that turns an input method on for an input of `purpose`, with its candidates at
/// the window's corner until the framework places them.
fn enable_request(purpose: ImePurpose) -> ImeEnableRequest {
    let capabilities = ImeCapabilities::new()
        .with_hint_and_purpose()
        .with_cursor_area();
    let data = ImeRequestData::default()
        .with_hint_and_purpose(ImeHint::NONE, purpose)
        .with_cursor_area(
            PhysicalPosition::new(0, 0).into(),
            PhysicalSize::new(0, 0).into(),
        );
    ImeEnableRequest::new(capabilities, data).expect("every capability asked for has its value")
}

/// The text input a view opened with `TextInputHost::start`, kept until `TextInputHost::stop`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveTextInput {
    multiline: bool,
    input_action: TextInputAction,
}

impl ActiveTextInput {
    fn new(configuration: &TextInputConfiguration) -> Self {
        ActiveTextInput {
            // By kind alone, as Flutter's plugin compares the type's name: a number type's
            // flags do not make it a multiline one.
            multiline: configuration.input_type.index == TextInputType::MULTILINE.index,
            input_action: configuration.input_action,
        }
    }

    /// What `FlutterTextInputPlugin` does for an unhandled key with this input active.
    fn key_effect(
        &self,
        key: &Key,
        text: Option<&str>,
        modifiers: ModifiersState,
    ) -> TextInputKeyEffect {
        // `NSTextInputContext` hands a Command or Control chord to the menu, never to
        // `insertText:`; winit's `text` still carries the bare character for those.
        if modifiers.control_key() || modifiers.meta_key() {
            return TextInputKeyEffect::None;
        }
        if *key == Key::Named(NamedKey::Enter) {
            return self.newline_effect();
        }
        match keys::character_of(text) {
            Some(text) => TextInputKeyEffect::Insert(text),
            None => TextInputKeyEffect::None,
        }
    }

    /// `FlutterTextInputPlugin.insertNewline:` — a multiline field with the newline action
    /// gets the "\n"; every field gets `performAction`.
    fn newline_effect(&self) -> TextInputKeyEffect {
        let inserts = self.multiline && self.input_action == TextInputAction::Newline;
        TextInputKeyEffect::Enter {
            insert: inserts.then(|| "\n".to_owned()),
            action: self.input_action,
        }
    }
}

/// What a key did to the text being edited.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Typed {
    Nothing,
    /// The editing value with the key's text in.
    Edited(TextEditingValue),
    /// Enter: the value with its newline, in a multiline field that takes one, and the field's
    /// action.
    Entered {
        edited: Option<TextEditingValue>,
        action: TextInputAction,
    },
}

/// What `FlutterTextInputPlugin` does with a key, before it touches the editing value.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TextInputKeyEffect {
    None,
    Insert(String),
    Enter {
        insert: Option<String>,
        action: TextInputAction,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_line() -> ActiveTextInput {
        ActiveTextInput {
            multiline: false,
            input_action: TextInputAction::Done,
        }
    }

    fn multiline() -> ActiveTextInput {
        ActiveTextInput {
            multiline: true,
            input_action: TextInputAction::Newline,
        }
    }

    #[test]
    fn a_character_key_inserts_its_text() {
        let effect = single_line().key_effect(
            &Key::Character("a".into()),
            Some("a"),
            ModifiersState::empty(),
        );
        assert_eq!(effect, TextInputKeyEffect::Insert("a".into()));
    }

    #[test]
    fn shift_and_option_still_type() {
        let effect = single_line().key_effect(
            &Key::Character("å".into()),
            Some("å"),
            ModifiersState::SHIFT | ModifiersState::ALT,
        );
        assert_eq!(effect, TextInputKeyEffect::Insert("å".into()));
    }

    #[test]
    fn command_and_control_chords_do_not_type() {
        for modifiers in [ModifiersState::META, ModifiersState::CONTROL] {
            let effect =
                single_line().key_effect(&Key::Character("c".into()), Some("c"), modifiers);
            assert_eq!(effect, TextInputKeyEffect::None);
        }
    }

    #[test]
    fn control_characters_do_not_type() {
        let effect = single_line().key_effect(
            &Key::Named(NamedKey::Backspace),
            Some("\u{8}"),
            ModifiersState::empty(),
        );
        assert_eq!(effect, TextInputKeyEffect::None);
    }

    #[test]
    fn enter_performs_the_action_and_only_a_multiline_field_gets_the_newline() {
        let enter = Key::Named(NamedKey::Enter);
        assert_eq!(
            single_line().key_effect(&enter, Some("\r"), ModifiersState::empty()),
            TextInputKeyEffect::Enter {
                insert: None,
                action: TextInputAction::Done,
            }
        );
        assert_eq!(
            multiline().key_effect(&enter, Some("\r"), ModifiersState::empty()),
            TextInputKeyEffect::Enter {
                insert: Some("\n".into()),
                action: TextInputAction::Newline,
            }
        );
    }
}

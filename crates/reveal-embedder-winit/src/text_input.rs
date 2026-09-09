//! The key half of Flutter's macOS `FlutterTextInputPlugin.handleKeyEvent`: a key the framework
//! did not handle reaches the active text input, which inserts its text or acts on Enter.
//!
//! winit reports `Ime::Commit` only for text an input method composed (`hasMarkedText`); plain
//! typing arrives as `KeyEvent::text`, so the host commits it here.

use reveal_embedder::{TextInputAction, TextInputConfiguration, TextInputType};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::keys;

/// The text input a view opened with `start_text_input`, kept until `stop_text_input`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActiveTextInput {
    multiline: bool,
    input_action: TextInputAction,
}

impl ActiveTextInput {
    pub(crate) fn new(configuration: &TextInputConfiguration) -> Self {
        ActiveTextInput {
            multiline: configuration.input_type.index == TextInputType::MULTILINE.index,
            input_action: configuration.input_action,
        }
    }

    /// What `FlutterTextInputPlugin` does for an unhandled key with this input active.
    pub(crate) fn key_effect(
        &self,
        key: &Key,
        text: Option<&str>,
        modifiers: ModifiersState,
    ) -> TextInputKeyEffect {
        // `NSTextInputContext` hands a Command or Control chord to the menu, never to
        // `insertText:`; winit's `text` still carries the bare character for those.
        if modifiers.control_key() || modifiers.super_key() {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TextInputKeyEffect {
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
        for modifiers in [ModifiersState::SUPER, ModifiersState::CONTROL] {
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

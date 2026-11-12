//! What the system's keyboard is told of a field, from the framework's configuration of it: the
//! part of Android's `EditorInfo` the app decides, as Flutter's Android `TextInputPlugin` fills it
//! (`inputTypeFromTextInputType`, `createInputConnection`), and the action an action key's code
//! names, as its `InputConnectionAdaptor.performEditorAction` reads it.
//!
//! The numbers are Android's `InputType` and `EditorInfo` constants; the text input view
//! (`text_input.rs`) hands them to the keyboard as they are.

use waft_embedder::{TextCapitalization, TextInputAction, TextInputConfiguration, TextInputType};

const TYPE_NULL: i32 = 0;
const TYPE_CLASS_TEXT: i32 = 1;
const TYPE_CLASS_NUMBER: i32 = 2;
const TYPE_CLASS_PHONE: i32 = 3;
const TYPE_CLASS_DATETIME: i32 = 4;
const TYPE_NUMBER_FLAG_SIGNED: i32 = 0x1000;
const TYPE_NUMBER_FLAG_DECIMAL: i32 = 0x2000;
const TYPE_NUMBER_VARIATION_PASSWORD: i32 = 0x10;
const TYPE_TEXT_FLAG_CAP_CHARACTERS: i32 = 0x1000;
const TYPE_TEXT_FLAG_CAP_WORDS: i32 = 0x2000;
const TYPE_TEXT_FLAG_CAP_SENTENCES: i32 = 0x4000;
const TYPE_TEXT_FLAG_AUTO_CORRECT: i32 = 0x8000;
const TYPE_TEXT_FLAG_MULTI_LINE: i32 = 0x2_0000;
const TYPE_TEXT_FLAG_NO_SUGGESTIONS: i32 = 0x8_0000;
const TYPE_TEXT_VARIATION_URI: i32 = 0x10;
const TYPE_TEXT_VARIATION_EMAIL_ADDRESS: i32 = 0x20;
const TYPE_TEXT_VARIATION_PERSON_NAME: i32 = 0x60;
const TYPE_TEXT_VARIATION_POSTAL_ADDRESS: i32 = 0x70;
const TYPE_TEXT_VARIATION_PASSWORD: i32 = 0x80;
const TYPE_TEXT_VARIATION_VISIBLE_PASSWORD: i32 = 0x90;

const IME_ACTION_UNSPECIFIED: i32 = 0;
const IME_ACTION_NONE: i32 = 1;
const IME_ACTION_GO: i32 = 2;
const IME_ACTION_SEARCH: i32 = 3;
const IME_ACTION_SEND: i32 = 4;
const IME_ACTION_NEXT: i32 = 5;
const IME_ACTION_DONE: i32 = 6;
const IME_ACTION_PREVIOUS: i32 = 7;
const IME_FLAG_NO_PERSONALIZED_LEARNING: i32 = 0x100_0000;
const IME_FLAG_NO_FULLSCREEN: i32 = 0x200_0000;

/// What the keyboard is told of a field: its `inputType`, its `imeOptions`, and the label of its
/// action key, where the field names one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EditorInfo {
    pub(crate) input_type: i32,
    pub(crate) ime_options: i32,
    pub(crate) action_label: Option<String>,
}

impl EditorInfo {
    /// The keyboard's view of a field configured as `configuration`. The keyboard never fills
    /// the screen, as Flutter's never does.
    pub(crate) fn of(configuration: &TextInputConfiguration) -> EditorInfo {
        let mut ime_options = IME_FLAG_NO_FULLSCREEN | ime_action_of(configuration.input_action);
        if !configuration.enable_ime_personalized_learning {
            ime_options |= IME_FLAG_NO_PERSONALIZED_LEARNING;
        }
        EditorInfo {
            input_type: input_type_of(configuration),
            ime_options,
            action_label: configuration.action_label.clone(),
        }
    }
}

/// The `inputType` of a field configured as `configuration`: its class and variation from its
/// input type, an obscured field's a password's with no suggestions, and its capitalization.
fn input_type_of(configuration: &TextInputConfiguration) -> i32 {
    let input_type = configuration.input_type;
    match input_type {
        TextInputType::DATETIME => return TYPE_CLASS_DATETIME,
        TextInputType::PHONE => return TYPE_CLASS_PHONE,
        TextInputType::NONE => return TYPE_NULL,
        _ if input_type.index == TextInputType::NUMBER.index => return number_type_of(input_type),
        _ => {}
    }
    text_variation_of(input_type)
        | suggestions_of(configuration)
        | capitalization_of(configuration.text_capitalization)
}

fn number_type_of(input_type: TextInputType) -> i32 {
    let mut number = TYPE_CLASS_NUMBER;
    if input_type.signed == Some(true) {
        number |= TYPE_NUMBER_FLAG_SIGNED;
    }
    if input_type.decimal == Some(true) {
        number |= TYPE_NUMBER_FLAG_DECIMAL;
    }
    if input_type.password == Some(true) {
        number |= TYPE_NUMBER_VARIATION_PASSWORD;
    }
    number
}

/// A text field's class with the variation, or the lines, its input type asks for.
fn text_variation_of(input_type: TextInputType) -> i32 {
    TYPE_CLASS_TEXT
        | match input_type {
            TextInputType::MULTILINE => TYPE_TEXT_FLAG_MULTI_LINE,
            TextInputType::EMAIL_ADDRESS | TextInputType::TWITTER => {
                TYPE_TEXT_VARIATION_EMAIL_ADDRESS
            }
            TextInputType::URL | TextInputType::WEB_SEARCH => TYPE_TEXT_VARIATION_URI,
            TextInputType::VISIBLE_PASSWORD => TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
            TextInputType::NAME => TYPE_TEXT_VARIATION_PERSON_NAME,
            TextInputType::STREET_ADDRESS => TYPE_TEXT_VARIATION_POSTAL_ADDRESS,
            _ => 0,
        }
}

/// An obscured field is a password's and suggests nothing; another corrects where asked, and one
/// that suggests nothing is taken for a visible password, which keyboards suggest nothing for.
fn suggestions_of(configuration: &TextInputConfiguration) -> i32 {
    if configuration.obscure_text {
        return TYPE_TEXT_FLAG_NO_SUGGESTIONS | TYPE_TEXT_VARIATION_PASSWORD;
    }
    let mut flags = 0;
    if configuration.autocorrect {
        flags |= TYPE_TEXT_FLAG_AUTO_CORRECT;
    }
    if !configuration.enable_suggestions {
        flags |= TYPE_TEXT_FLAG_NO_SUGGESTIONS | TYPE_TEXT_VARIATION_VISIBLE_PASSWORD;
    }
    flags
}

fn capitalization_of(capitalization: TextCapitalization) -> i32 {
    match capitalization {
        TextCapitalization::Characters => TYPE_TEXT_FLAG_CAP_CHARACTERS,
        TextCapitalization::Words => TYPE_TEXT_FLAG_CAP_WORDS,
        TextCapitalization::Sentences => TYPE_TEXT_FLAG_CAP_SENTENCES,
        TextCapitalization::None => 0,
    }
}

/// The action key's code for `action`; the actions Android has no key for are left to the
/// keyboard, as Flutter's `inputActionFromTextInputAction` leaves them.
fn ime_action_of(action: TextInputAction) -> i32 {
    match action {
        TextInputAction::None | TextInputAction::Newline => IME_ACTION_NONE,
        TextInputAction::Done => IME_ACTION_DONE,
        TextInputAction::Go => IME_ACTION_GO,
        TextInputAction::Search => IME_ACTION_SEARCH,
        TextInputAction::Send => IME_ACTION_SEND,
        TextInputAction::Next => IME_ACTION_NEXT,
        TextInputAction::Previous => IME_ACTION_PREVIOUS,
        TextInputAction::Unspecified
        | TextInputAction::ContinueAction
        | TextInputAction::Join
        | TextInputAction::Route
        | TextInputAction::EmergencyCall => IME_ACTION_UNSPECIFIED,
    }
}

/// The action the action key's `code` names: no action is a new line, and an action Android does
/// not name is done, as Flutter's `performEditorAction` reads them.
pub(crate) fn action_of(code: i32) -> TextInputAction {
    match code {
        IME_ACTION_NONE => TextInputAction::Newline,
        IME_ACTION_UNSPECIFIED => TextInputAction::Unspecified,
        IME_ACTION_GO => TextInputAction::Go,
        IME_ACTION_SEARCH => TextInputAction::Search,
        IME_ACTION_SEND => TextInputAction::Send,
        IME_ACTION_NEXT => TextInputAction::Next,
        IME_ACTION_PREVIOUS => TextInputAction::Previous,
        _ => TextInputAction::Done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_field_corrects_and_acts_done_without_the_full_screen() {
        let info = EditorInfo::of(&TextInputConfiguration::builder().build());
        assert_eq!(
            info.input_type,
            TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_AUTO_CORRECT
        );
        assert_eq!(info.ime_options, IME_FLAG_NO_FULLSCREEN | IME_ACTION_DONE);
        assert_eq!(info.action_label, None);
    }

    #[test]
    fn an_obscured_field_is_a_password_that_suggests_nothing() {
        let info = EditorInfo::of(&TextInputConfiguration::builder().obscure_text(true).build());
        assert_eq!(
            info.input_type,
            TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_PASSWORD | TYPE_TEXT_FLAG_NO_SUGGESTIONS
        );
    }

    #[test]
    fn a_multiline_field_capitalizes_sentences_and_has_no_action() {
        let info = EditorInfo::of(
            &TextInputConfiguration::builder()
                .input_type(TextInputType::MULTILINE)
                .input_action(TextInputAction::Newline)
                .text_capitalization(TextCapitalization::Sentences)
                .autocorrect(false)
                .build(),
        );
        assert_eq!(
            info.input_type,
            TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_MULTI_LINE | TYPE_TEXT_FLAG_CAP_SENTENCES
        );
        assert_eq!(info.ime_options & 0xff, IME_ACTION_NONE);
    }

    #[test]
    fn a_number_field_carries_its_options() {
        let input_type = TextInputType::number_with_options()
            .signed(true)
            .decimal(true);
        let info = EditorInfo::of(
            &TextInputConfiguration::builder()
                .input_type(input_type)
                .build(),
        );
        assert_eq!(
            info.input_type,
            TYPE_CLASS_NUMBER | TYPE_NUMBER_FLAG_SIGNED | TYPE_NUMBER_FLAG_DECIMAL
        );
    }

    #[test]
    fn no_keyboard_is_the_null_type() {
        let info = EditorInfo::of(
            &TextInputConfiguration::builder()
                .input_type(TextInputType::NONE)
                .build(),
        );
        assert_eq!(info.input_type, TYPE_NULL);
    }

    #[test]
    fn action_codes_read_back_as_the_actions_that_set_them() {
        for action in [
            TextInputAction::Done,
            TextInputAction::Go,
            TextInputAction::Search,
            TextInputAction::Send,
            TextInputAction::Next,
            TextInputAction::Previous,
            TextInputAction::Newline,
            TextInputAction::Unspecified,
        ] {
            assert_eq!(action_of(ime_action_of(action)), action);
        }
    }
}

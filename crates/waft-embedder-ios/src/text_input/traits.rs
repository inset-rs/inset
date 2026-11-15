//! A field's configuration as the keyboard's traits, as Flutter's `FlutterTextInputView`'s
//! `configureWithDictionary:` sets them (`ToUIKeyboardType`, `ToUIReturnKeyType`,
//! `ToUITextAutoCapitalizationType`, `ToUITextContentType`).

// UIKit's content type constants need `unsafe` to read; this module writes it.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2_foundation::{NSCopying, NSString};
use objc2_ui_kit::{
    UIKeyboardAppearance, UIKeyboardType, UIReturnKeyType, UITextAutocapitalizationType,
    UITextAutocorrectionType, UITextContentType, UITextContentTypeAddressCity,
    UITextContentTypeAddressCityAndState, UITextContentTypeAddressState,
    UITextContentTypeCountryName, UITextContentTypeCreditCardNumber, UITextContentTypeEmailAddress,
    UITextContentTypeFamilyName, UITextContentTypeFullStreetAddress, UITextContentTypeGivenName,
    UITextContentTypeJobTitle, UITextContentTypeLocation, UITextContentTypeMiddleName,
    UITextContentTypeName, UITextContentTypeNamePrefix, UITextContentTypeNameSuffix,
    UITextContentTypeNewPassword, UITextContentTypeNickname, UITextContentTypeOneTimeCode,
    UITextContentTypeOrganizationName, UITextContentTypePassword, UITextContentTypePostalCode,
    UITextContentTypeStreetAddressLine1, UITextContentTypeStreetAddressLine2,
    UITextContentTypeSublocality, UITextContentTypeTelephoneNumber, UITextSmartDashesType,
    UITextSmartQuotesType, UITextSpellCheckingType,
};
use waft_embedder::{
    Brightness, SmartDashesType, SmartQuotesType, TextCapitalization, TextInputAction,
    TextInputConfiguration, TextInputType,
};

/// What the keyboard is configured with for one field.
#[derive(Clone)]
pub(crate) struct Traits {
    pub(crate) secure_text_entry: bool,
    pub(crate) is_system_keyboard_enabled: bool,
    pub(crate) enable_interactive_selection: bool,
    pub(crate) keyboard_type: UIKeyboardType,
    pub(crate) return_key_type: UIReturnKeyType,
    pub(crate) autocapitalization_type: UITextAutocapitalizationType,
    pub(crate) smart_dashes_type: UITextSmartDashesType,
    pub(crate) smart_quotes_type: UITextSmartQuotesType,
    pub(crate) keyboard_appearance: UIKeyboardAppearance,
    pub(crate) autocorrection_type: UITextAutocorrectionType,
    pub(crate) spell_checking_type: UITextSpellCheckingType,
    /// Empty where the field asks no autofill, as Flutter sets it.
    pub(crate) text_content_type: Retained<NSString>,
}

impl Traits {
    /// The traits of a field the framework has not configured: the defaults.
    pub(crate) fn unconfigured() -> Traits {
        Traits::of(&TextInputConfiguration::builder().build())
    }

    /// `configureWithDictionary:`'s traits for `configuration`.
    pub(crate) fn of(configuration: &TextInputConfiguration) -> Traits {
        let autocorrect_is_disabled = !configuration.autocorrect;
        let autofill = &configuration.autofill_configuration;
        Traits {
            secure_text_entry: configuration.obscure_text,
            is_system_keyboard_enabled: configuration.input_type.index != TextInputType::NONE.index,
            enable_interactive_selection: configuration.enable_interactive_selection,
            keyboard_type: keyboard_type(configuration.input_type),
            return_key_type: return_key_type(configuration.input_action),
            autocapitalization_type: autocapitalization_type(configuration.text_capitalization),
            smart_dashes_type: match configuration.smart_dashes_type {
                SmartDashesType::Disabled => UITextSmartDashesType::No,
                SmartDashesType::Enabled => UITextSmartDashesType::Yes,
            },
            smart_quotes_type: match configuration.smart_quotes_type {
                SmartQuotesType::Disabled => UITextSmartQuotesType::No,
                SmartQuotesType::Enabled => UITextSmartQuotesType::Yes,
            },
            keyboard_appearance: match configuration.keyboard_appearance {
                Brightness::Dark => UIKeyboardAppearance::Dark,
                Brightness::Light => UIKeyboardAppearance::Light,
            },
            autocorrection_type: if autocorrect_is_disabled {
                UITextAutocorrectionType::No
            } else {
                UITextAutocorrectionType::Default
            },
            spell_checking_type: if autocorrect_is_disabled {
                UITextSpellCheckingType::No
            } else {
                UITextSpellCheckingType::Default
            },
            text_content_type: if autofill.enabled {
                text_content_type(&autofill.autofill_hints)
            } else {
                NSString::new()
            },
        }
    }
}

/// `ToUIKeyboardType`, by the input type's index in Flutter's `TextInputType.values`.
fn keyboard_type(input_type: TextInputType) -> UIKeyboardType {
    match input_type.index {
        // datetime
        4 => UIKeyboardType::NumbersAndPunctuation,
        // emailAddress
        5 => UIKeyboardType::EmailAddress,
        // name
        8 => UIKeyboardType::NamePhonePad,
        // number
        2 if input_type.signed == Some(true) => UIKeyboardType::NumbersAndPunctuation,
        2 if input_type.decimal == Some(true) => UIKeyboardType::DecimalPad,
        2 => UIKeyboardType::NumberPad,
        // phone
        3 => UIKeyboardType::PhonePad,
        // url
        6 => UIKeyboardType::URL,
        // visiblePassword
        7 => UIKeyboardType::ASCIICapable,
        // webSearch
        11 => UIKeyboardType::WebSearch,
        // twitter
        12 => UIKeyboardType::Twitter,
        // address, multiline, text, and any other
        _ => UIKeyboardType::Default,
    }
}

/// `ToUIReturnKeyType`.
fn return_key_type(action: TextInputAction) -> UIReturnKeyType {
    match action {
        TextInputAction::Done => UIReturnKeyType::Done,
        TextInputAction::Go => UIReturnKeyType::Go,
        TextInputAction::Send => UIReturnKeyType::Send,
        TextInputAction::Search => UIReturnKeyType::Search,
        TextInputAction::Next => UIReturnKeyType::Next,
        TextInputAction::ContinueAction => UIReturnKeyType::Continue,
        TextInputAction::Join => UIReturnKeyType::Join,
        TextInputAction::Route => UIReturnKeyType::Route,
        TextInputAction::EmergencyCall => UIReturnKeyType::EmergencyCall,
        _ => UIReturnKeyType::Default,
    }
}

/// The action the return key is, as `shouldChangeTextInRange:replacementText:` sends it.
pub(crate) fn action_of(return_key_type: UIReturnKeyType) -> TextInputAction {
    match return_key_type {
        UIReturnKeyType::Done => TextInputAction::Done,
        UIReturnKeyType::Go => TextInputAction::Go,
        UIReturnKeyType::Send => TextInputAction::Send,
        UIReturnKeyType::Search | UIReturnKeyType::Google | UIReturnKeyType::Yahoo => {
            TextInputAction::Search
        }
        UIReturnKeyType::Next => TextInputAction::Next,
        UIReturnKeyType::Continue => TextInputAction::ContinueAction,
        UIReturnKeyType::Join => TextInputAction::Join,
        UIReturnKeyType::Route => TextInputAction::Route,
        UIReturnKeyType::EmergencyCall => TextInputAction::EmergencyCall,
        _ => TextInputAction::Unspecified,
    }
}

/// `ToUITextAutoCapitalizationType`.
fn autocapitalization_type(capitalization: TextCapitalization) -> UITextAutocapitalizationType {
    match capitalization {
        TextCapitalization::Characters => UITextAutocapitalizationType::AllCharacters,
        TextCapitalization::Sentences => UITextAutocapitalizationType::Sentences,
        TextCapitalization::Words => UITextAutocapitalizationType::Words,
        TextCapitalization::None => UITextAutocapitalizationType::None,
    }
}

/// `ToUITextContentType`: the first hint, as UIKit's own content type where it names one, and as
/// itself otherwise.
fn text_content_type(hints: &[String]) -> Retained<NSString> {
    let Some(hint) = hints.first() else {
        return NSString::new();
    };
    // SAFETY: constants UIKit exports for the life of the process.
    let uikit: Option<&UITextContentType> = unsafe {
        match hint.as_str() {
            "addressCityAndState" => Some(UITextContentTypeAddressCityAndState),
            "addressState" => Some(UITextContentTypeAddressState),
            "addressCity" => Some(UITextContentTypeAddressCity),
            "sublocality" => Some(UITextContentTypeSublocality),
            "streetAddressLine1" => Some(UITextContentTypeStreetAddressLine1),
            "streetAddressLine2" => Some(UITextContentTypeStreetAddressLine2),
            "countryName" => Some(UITextContentTypeCountryName),
            "fullStreetAddress" => Some(UITextContentTypeFullStreetAddress),
            "postalCode" => Some(UITextContentTypePostalCode),
            "location" => Some(UITextContentTypeLocation),
            "creditCardNumber" => Some(UITextContentTypeCreditCardNumber),
            "email" => Some(UITextContentTypeEmailAddress),
            "jobTitle" => Some(UITextContentTypeJobTitle),
            "givenName" => Some(UITextContentTypeGivenName),
            "middleName" => Some(UITextContentTypeMiddleName),
            "familyName" => Some(UITextContentTypeFamilyName),
            "name" => Some(UITextContentTypeName),
            "namePrefix" => Some(UITextContentTypeNamePrefix),
            "nameSuffix" => Some(UITextContentTypeNameSuffix),
            "nickname" => Some(UITextContentTypeNickname),
            "organizationName" => Some(UITextContentTypeOrganizationName),
            "telephoneNumber" => Some(UITextContentTypeTelephoneNumber),
            "password" => Some(UITextContentTypePassword),
            "oneTimeCode" => Some(UITextContentTypeOneTimeCode),
            "newPassword" => Some(UITextContentTypeNewPassword),
            _ => None,
        }
    };
    match uikit {
        Some(content_type) => content_type.copy(),
        None => NSString::from_str(hint),
    }
}

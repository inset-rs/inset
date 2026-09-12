//! Flutter counterpart: `services/text_input.dart` (value types and
//! [`TextInputConfiguration`]).
//!
//! Lives here because [`crate::View`] and [`crate::EmbedderClient`] name these
//! types. The connection and client stay in `inset-services`.

use std::fmt;

use crate::text_editing::TextSelection;
use crate::{Brightness, Locale, Offset, TextAffinity, TextPosition, TextRange};

/// Indicates how to handle the intelligent replacement of dashes in text input.
///
/// See also:
///
///  * `TextField.smartDashesType`
///  * `CupertinoTextField.smartDashesType`
///  * `EditableText.smartDashesType`
///  * [`SmartQuotesType`]
///  * <https://developer.apple.com/documentation/uikit/uitextinputtraits>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SmartDashesType {
    /// Smart dashes is disabled.
    ///
    /// This corresponds to the
    /// ["no" value of UITextSmartDashesType](https://developer.apple.com/documentation/uikit/uitextsmartdashestype/no).
    Disabled,

    /// Smart dashes is enabled.
    ///
    /// This corresponds to the
    /// ["yes" value of UITextSmartDashesType](https://developer.apple.com/documentation/uikit/uitextsmartdashestype/yes).
    Enabled,
}

/// Indicates how to handle the intelligent replacement of quotes in text input.
///
/// See also:
///
///  * `TextField.smartQuotesType`
///  * `CupertinoTextField.smartQuotesType`
///  * `EditableText.smartQuotesType`
///  * <https://developer.apple.com/documentation/uikit/uitextinputtraits>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SmartQuotesType {
    /// Smart quotes is disabled.
    ///
    /// This corresponds to the
    /// ["no" value of UITextSmartQuotesType](https://developer.apple.com/documentation/uikit/uitextsmartquotestype/no).
    Disabled,

    /// Smart quotes is enabled.
    ///
    /// This corresponds to the
    /// ["yes" value of UITextSmartQuotesType](https://developer.apple.com/documentation/uikit/uitextsmartquotestype/yes).
    Enabled,
}

/// The type of information for which to optimize the text input control.
///
/// On Android, behavior may vary across device and keyboard provider.
///
/// This class stays as close to `Enum` interface as possible, and allows
/// for additional flags for some input types. For example, numeric input
/// can specify whether it supports decimal numbers and/or signed numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextInputType {
    /// Enum value index, corresponds to one of the [`VALUES`](Self::VALUES).
    pub index: i32,
    /// The number is signed, allowing a positive or negative sign at the start.
    ///
    /// This flag is only used for the [`NUMBER`](Self::NUMBER) input type, otherwise `None`.
    /// Use `TextInputType::number_with_options().signed(true)` to set this.
    pub signed: Option<bool>,
    /// The number is decimal, allowing a decimal point to provide fractional.
    ///
    /// This flag is only used for the [`NUMBER`](Self::NUMBER) input type, otherwise `None`.
    /// Use `TextInputType::number_with_options().decimal(true)` to set this.
    pub decimal: Option<bool>,
    /// The number is a password.
    ///
    /// This flag is only used for the [`NUMBER`](Self::NUMBER) input type, otherwise `None`.
    /// Use `TextInputType::number_with_options().password(true)` to set this.
    ///
    /// On Android, this adds
    /// [TYPE_NUMBER_VARIATION_PASSWORD](https://developer.android.com/reference/android/text/InputType#TYPE_NUMBER_VARIATION_PASSWORD)
    /// to the input type. It has no effect on iOS, web, or desktop platforms.
    pub password: Option<bool>,
}

impl TextInputType {
    const fn named(index: i32) -> TextInputType {
        TextInputType {
            index,
            signed: None,
            decimal: None,
            password: None,
        }
    }

    /// Optimize for textual information.
    ///
    /// Requests the default platform keyboard.
    pub const TEXT: TextInputType = TextInputType::named(0);

    /// Optimize for multiline textual information.
    ///
    /// Requests the default platform keyboard, but accepts newlines when the
    /// enter key is pressed. This is the input type used for all multiline text
    /// fields.
    pub const MULTILINE: TextInputType = TextInputType::named(1);

    /// Optimize for unsigned numerical information without a decimal point.
    ///
    /// Requests a default keyboard with ready access to the number keys.
    /// Additional options, such as decimal point and/or positive/negative
    /// signs, can be requested using [`number_with_options`](Self::number_with_options).
    pub const NUMBER: TextInputType = TextInputType::number_with_options();

    /// Optimize for telephone numbers.
    ///
    /// Requests a keyboard with ready access to the number keys, "*", and "#".
    pub const PHONE: TextInputType = TextInputType::named(3);

    /// Optimize for date and time information.
    ///
    /// On iOS, requests the default keyboard.
    ///
    /// On Android, requests a keyboard with ready access to the number keys,
    /// ":", and "-".
    pub const DATETIME: TextInputType = TextInputType::named(4);

    /// Optimize for email addresses.
    ///
    /// Requests a keyboard with ready access to the "@" and "." keys.
    pub const EMAIL_ADDRESS: TextInputType = TextInputType::named(5);

    /// Optimize for URLs.
    ///
    /// Requests a keyboard with ready access to the "/" and "." keys.
    pub const URL: TextInputType = TextInputType::named(6);

    /// Optimize for passwords that are visible to the user.
    ///
    /// Requests a keyboard with ready access to both letters and numbers.
    pub const VISIBLE_PASSWORD: TextInputType = TextInputType::named(7);

    /// Optimized for a person's name.
    ///
    /// On iOS, requests the
    /// [UIKeyboardType.namePhonePad](https://developer.apple.com/documentation/uikit/uikeyboardtype/namephonepad)
    /// keyboard, a keyboard optimized for entering a person’s name or phone number.
    /// Does not support auto-capitalization.
    ///
    /// On Android, requests a keyboard optimized for
    /// [TYPE_TEXT_VARIATION_PERSON_NAME](https://developer.android.com/reference/android/text/InputType#TYPE_TEXT_VARIATION_PERSON_NAME).
    pub const NAME: TextInputType = TextInputType::named(8);

    /// Optimized for postal mailing addresses.
    ///
    /// On iOS, requests the default keyboard.
    ///
    /// On Android, requests a keyboard optimized for
    /// [TYPE_TEXT_VARIATION_POSTAL_ADDRESS](https://developer.android.com/reference/android/text/InputType#TYPE_TEXT_VARIATION_POSTAL_ADDRESS).
    pub const STREET_ADDRESS: TextInputType = TextInputType::named(9);

    /// Prevent the OS from showing the on-screen virtual keyboard.
    pub const NONE: TextInputType = TextInputType::named(10);

    /// Optimized for web searches.
    ///
    /// Requests a keyboard that includes keys useful for web searches as well as URLs.
    ///
    /// On iOS, requests a default keyboard with ready access to the "." key. In contrast to
    /// [`URL`](Self::URL), a space bar is available.
    ///
    /// On Android this is remapped to the [`URL`](Self::URL) keyboard type as it always shows a space bar.
    pub const WEB_SEARCH: TextInputType = TextInputType::named(11);

    /// Optimized for social media.
    ///
    /// Requests a keyboard that includes keys useful for handles and tags.
    ///
    /// On iOS, requests a default keyboard with ready access to the "@" and "#" keys.
    ///
    /// On Android this is remapped to the [`EMAIL_ADDRESS`](Self::EMAIL_ADDRESS) keyboard type as it always shows the "@" key.
    pub const TWITTER: TextInputType = TextInputType::named(12);

    /// All possible enum values.
    pub const VALUES: &[TextInputType] = &[
        Self::TEXT,
        Self::MULTILINE,
        Self::NUMBER,
        Self::PHONE,
        Self::DATETIME,
        Self::EMAIL_ADDRESS,
        Self::URL,
        Self::VISIBLE_PASSWORD,
        Self::NAME,
        Self::STREET_ADDRESS,
        Self::NONE,
        Self::WEB_SEARCH,
        Self::TWITTER,
    ];

    const NAMES: &[&str] = &[
        "text",
        "multiline",
        "number",
        "phone",
        "datetime",
        "emailAddress",
        "url",
        "visiblePassword",
        "name",
        "address",
        "none",
        "webSearch",
        "twitter",
    ];

    /// Optimize for numerical information.
    ///
    /// Requests a numeric keyboard with additional settings.
    /// The [`signed`](Self::signed), [`decimal`](Self::decimal), and [`password`](Self::password)
    /// parameters are optional.
    pub const fn number_with_options() -> TextInputType {
        TextInputType {
            index: 2,
            signed: Some(false),
            decimal: Some(false),
            password: Some(false),
        }
    }

    /// Dart `TextInputType.numberWithOptions(signed:)`.
    pub fn signed(mut self, signed: bool) -> TextInputType {
        self.signed = Some(signed);
        self
    }

    /// Dart `TextInputType.numberWithOptions(decimal:)`.
    pub fn decimal(mut self, decimal: bool) -> TextInputType {
        self.decimal = Some(decimal);
        self
    }

    /// Dart `TextInputType.numberWithOptions(password:)`.
    pub fn password(mut self, password: bool) -> TextInputType {
        self.password = Some(password);
        self
    }

    fn name(self) -> String {
        format!("TextInputType.{}", Self::NAMES[self.index as usize])
    }
}

impl fmt::Display for TextInputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TextInputType(name: {}, signed: {:?}, decimal: {:?}, password: {:?})",
            self.name(),
            self.signed,
            self.decimal,
            self.password
        )
    }
}

/// An action the user has requested the text input control to perform.
///
/// Each action represents a logical meaning, and also configures the soft
/// keyboard to display a certain kind of action button. The visual appearance
/// of the action button might differ between versions of the same OS.
///
/// Despite the logical meaning of each action, choosing a particular
/// [`TextInputAction`] does not necessarily cause any specific behavior to
/// happen, other than changing the focus when appropriate. It is up to the
/// developer to ensure that the behavior that occurs when an action button is
/// pressed is appropriate for the action button chosen.
///
/// For example: If the user presses the keyboard action button on iOS when it
/// reads "Emergency Call", the result should not be a focus change to the next
/// TextField. This behavior is not logically appropriate for a button that says
/// "Emergency Call".
///
/// See `EditableText` for more information about customizing action button
/// behavior.
///
/// Most [`TextInputAction`]s are supported equally by both Android and iOS.
/// However, there is not a complete, direct mapping between Android's IME input
/// types and iOS's keyboard return types. Therefore, some [`TextInputAction`]s
/// are inappropriate for one of the platforms. If a developer chooses an
/// inappropriate [`TextInputAction`] when running in debug mode, an error will be
/// thrown. If the same thing is done in release mode, then instead of sending
/// the inappropriate value, Android will use "unspecified" on the platform
/// side and iOS will use "default" on the platform side.
///
/// See also:
///
///  * `TextInput`, which configures the platform's keyboard setup.
///  * `EditableText`, which invokes callbacks when the action button is pressed.
//
// This class has been cloned to `flutter_driver/lib/src/common/action.dart` as `TextInputAction`,
// and must be kept in sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextInputAction {
    /// Logical meaning: There is no relevant input action for the current input
    /// source, e.g., `TextField`.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_NONE". The keyboard setup
    /// is decided by the OS. The keyboard will likely show a return key.
    ///
    /// iOS: iOS does not have a keyboard return type of "none." It is
    /// inappropriate to choose this [`TextInputAction`] when running on iOS.
    None,

    /// Logical meaning: Let the OS decide which action is most appropriate.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_UNSPECIFIED". The OS chooses
    /// which keyboard action to display. The decision will likely be a done
    /// button or a return key.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyDefault". The title displayed in
    /// the action button is "return".
    Unspecified,

    /// Logical meaning: The user is done providing input to a group of inputs
    /// (like a form). Some kind of finalization behavior should now take place.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_DONE". The OS displays a
    /// button that represents completion, e.g., a checkmark button.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyDone". The title displayed in
    /// the action button is "Done".
    Done,

    /// Logical meaning: The user has entered some text that represents a
    /// destination, e.g., a restaurant name. The "go" button is intended to take
    /// the user to a part of the app that corresponds to this destination.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_GO". The OS displays a
    /// button that represents taking "the user to the target of the text they
    /// typed", e.g., a right-facing arrow button.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyGo". The title displayed in the
    /// action button is "Go".
    Go,

    /// Logical meaning: Execute a search query.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_SEARCH". The OS displays a
    /// button that represents a search, e.g., a magnifying glass button.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeySearch". The title displayed in
    /// the action button is "Search".
    Search,

    /// Logical meaning: Sends something that the user has composed, e.g., an
    /// email or a text message.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_SEND". The OS displays a
    /// button that represents sending something, e.g., a paper plane button.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeySend". The title displayed in the
    /// action button is "Send".
    Send,

    /// Logical meaning: The user is done with the current input source and wants
    /// to move to the next one.
    ///
    /// Moves the focus to the next focusable item in the same `FocusScope`.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_NEXT". The OS displays a
    /// button that represents moving forward, e.g., a right-facing arrow button.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyNext". The title displayed in the
    /// action button is "Next".
    Next,

    /// Logical meaning: The user wishes to return to the previous input source
    /// in the group, e.g., a form with multiple `TextField`s.
    ///
    /// Moves the focus to the previous focusable item in the same `FocusScope`.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_PREVIOUS". The OS displays a
    /// button that represents moving backward, e.g., a left-facing arrow button.
    ///
    /// iOS: iOS does not have a keyboard return type of "previous." It is
    /// inappropriate to choose this [`TextInputAction`] when running on iOS.
    Previous,

    /// Logical meaning: In iOS apps, it is common for a "Back" button and
    /// "Continue" button to appear at the top of the screen. However, when the
    /// keyboard is open, these buttons are often hidden off-screen. Therefore,
    /// the purpose of the "Continue" return key on iOS is to make the "Continue"
    /// button available when the user is entering text.
    ///
    /// Historical context aside, [`ContinueAction`](TextInputAction::ContinueAction) can be used any
    /// time that the term "Continue" seems most appropriate for the given action.
    ///
    /// Android: Android does not have an IME input type of "continue." It is
    /// inappropriate to choose this [`TextInputAction`] when running on Android.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyContinue". The title displayed in the
    /// action button is "Continue". This action is only available on iOS 9.0+.
    ///
    /// The reason that this value has "Action" post-fixed to it is because
    /// "continue" is a reserved word in Dart, as well as many other languages.
    ContinueAction,

    /// Logical meaning: The user wants to join something, e.g., a wireless
    /// network.
    ///
    /// Android: Android does not have an IME input type of "join." It is
    /// inappropriate to choose this [`TextInputAction`] when running on Android.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyJoin". The title displayed in the
    /// action button is "Join".
    Join,

    /// Logical meaning: The user wants routing options, e.g., driving directions.
    ///
    /// Android: Android does not have an IME input type of "route." It is
    /// inappropriate to choose this [`TextInputAction`] when running on Android.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyRoute". The title displayed in the
    /// action button is "Route".
    Route,

    /// Logical meaning: Initiate a call to emergency services.
    ///
    /// Android: Android does not have an IME input type of "emergencyCall." It is
    /// inappropriate to choose this [`TextInputAction`] when running on Android.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyEmergencyCall". The title displayed
    /// in the action button is "Emergency Call".
    EmergencyCall,

    /// Logical meaning: Insert a newline character in the focused text input,
    /// e.g., `TextField`.
    ///
    /// Android: Corresponds to Android's "IME_ACTION_NONE". The OS displays a
    /// button that represents a new line, e.g., a carriage return button.
    ///
    /// iOS: Corresponds to iOS's "UIReturnKeyDefault". The title displayed in
    /// the action button is "return".
    ///
    /// The term [`Newline`](TextInputAction::Newline) exists in Flutter but not in Android
    /// or iOS. The reason for introducing this term is so that developers can
    /// achieve the common result of inserting new lines without needing to
    /// understand the various IME actions on Android and return keys on iOS.
    /// Thus, [`Newline`](TextInputAction::Newline) is a convenience term that alleviates the
    /// need to understand the underlying platforms to achieve this common behavior.
    Newline,
}

/// Configures how the platform keyboard will select an uppercase or
/// lowercase keyboard.
///
/// Only supports text keyboards, other keyboard types will ignore this
/// configuration. Capitalization is locale-aware.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextCapitalization {
    /// Defaults to an uppercase keyboard for the first letter of each word.
    ///
    /// Corresponds to `InputType.TYPE_TEXT_FLAG_CAP_WORDS` on Android, and
    /// `UITextAutocapitalizationTypeWords` on iOS.
    Words,

    /// Defaults to an uppercase keyboard for the first letter of each sentence.
    ///
    /// Corresponds to `InputType.TYPE_TEXT_FLAG_CAP_SENTENCES` on Android, and
    /// `UITextAutocapitalizationTypeSentences` on iOS.
    Sentences,

    /// Defaults to an uppercase keyboard for each character.
    ///
    /// Corresponds to `InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS` on Android, and
    /// `UITextAutocapitalizationTypeAllCharacters` on iOS.
    Characters,

    /// Defaults to a lowercase keyboard.
    None,
}

/// A collection of autofill related information that represents an `AutofillClient`.
///
/// Typically used in [`TextInputConfiguration::autofill_configuration`].
///
/// Lives here with [`TextInputConfiguration`] because [`crate::View`] names that
/// type. Flutter's file is `services/autofill.dart`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AutofillConfiguration {
    /// Whether autofill should be enabled for the `AutofillClient`.
    ///
    /// To retrieve a disabled [`AutofillConfiguration`], use [`DISABLED`](Self::DISABLED).
    pub enabled: bool,
    /// A string that uniquely identifies the current `AutofillClient`.
    ///
    /// The identifier needs to be unique within the `AutofillScope` for the
    /// `AutofillClient` to receive the correct autofill value.
    pub unique_identifier: String,
    /// A list of strings that helps the autofill service identify the type of the
    /// `AutofillClient`.
    ///
    /// See `AutofillHints` (`services/autofill.dart`).
    pub autofill_hints: Vec<String>,
    /// The current [`TextEditingValue`] of the `AutofillClient`.
    pub current_editing_value: TextEditingValue,
    /// The optional hint text placed on the view that typically suggests what
    /// sort of input the field accepts, for example "enter your password here".
    ///
    /// If the developer does not specify any [`autofill_hints`](Self::autofill_hints), the
    /// [`hint_text`](Self::hint_text) can be a useful indication to the platform autofill service.
    pub hint_text: Option<String>,
}

impl AutofillConfiguration {
    /// An [`AutofillConfiguration`] that indicates the `AutofillClient` does not
    /// wish to be autofilled.
    pub const DISABLED: AutofillConfiguration = AutofillConfiguration {
        enabled: false,
        unique_identifier: String::new(),
        autofill_hints: Vec::new(),
        current_editing_value: TextEditingValue::EMPTY,
        hint_text: None,
    };

    /// Creates autofill related configuration information that can be sent to the
    /// platform.
    pub fn new(
        unique_identifier: impl Into<String>,
        autofill_hints: Vec<String>,
        current_editing_value: TextEditingValue,
    ) -> AutofillConfiguration {
        AutofillConfiguration {
            enabled: true,
            unique_identifier: unique_identifier.into(),
            autofill_hints,
            current_editing_value,
            hint_text: None,
        }
    }

    /// Dart `AutofillConfiguration(hintText:)`.
    pub fn hint_text(mut self, hint_text: impl Into<String>) -> AutofillConfiguration {
        self.hint_text = Some(hint_text.into());
        self
    }
}

impl fmt::Display for AutofillConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AutofillConfiguration(enabled: {}, uniqueIdentifier: {}, autofillHints: {:?}, currentEditingValue: {}",
            self.enabled, self.unique_identifier, self.autofill_hints, self.current_editing_value
        )?;
        if let Some(hint_text) = &self.hint_text {
            write!(f, ", hintText: {hint_text}")?;
        }
        write!(f, ")")
    }
}

/// Controls the visual appearance of the text input control.
///
/// Many [`TextInputAction`]s are common between Android and iOS. However, if an
/// [`input_action`](Self::input_action) is provided that is not supported by the current
/// platform in debug mode, an error will be thrown when the corresponding
/// text input is attached. For example, providing iOS's "emergencyCall"
/// action when running on an Android device will result in an error when in
/// debug mode. In release mode, incompatible [`TextInputAction`]s are replaced
/// either with "unspecified" on Android, or "default" on iOS. Appropriate
/// [`input_action`](Self::input_action)s can be chosen by checking the current platform and then
/// selecting the appropriate action.
///
/// See also:
///
///  * `TextInput.attach`
///  * [`TextInputAction`]
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputConfiguration {
    /// The ID of the view that the text input belongs to.
    pub view_id: Option<i32>,
    /// The type of information for which to optimize the text input control.
    pub input_type: TextInputType,
    /// Whether the text field can be edited or not.
    ///
    /// Defaults to false.
    pub read_only: bool,
    /// Whether to hide the text being edited (e.g., for passwords).
    ///
    /// Defaults to false.
    pub obscure_text: bool,
    /// Whether to enable autocorrection.
    ///
    /// Defaults to true.
    pub autocorrect: bool,
    /// Whether to allow the platform to automatically format dashes.
    pub smart_dashes_type: SmartDashesType,
    /// Whether to allow the platform to automatically format quotes.
    pub smart_quotes_type: SmartQuotesType,
    /// Whether to show input suggestions as the user types.
    ///
    /// Defaults to true.
    pub enable_suggestions: bool,
    /// Whether a user can change its selection.
    ///
    /// Defaults to true.
    pub enable_interactive_selection: bool,
    /// What text to display in the text input control's action button.
    pub action_label: Option<String>,
    /// What kind of action to request for the action button on the IME.
    pub input_action: TextInputAction,
    /// Specifies how platforms may automatically capitalize text entered by the
    /// user.
    ///
    /// Defaults to [`TextCapitalization::None`].
    pub text_capitalization: TextCapitalization,
    /// The appearance of the keyboard.
    ///
    /// This setting is only honored on iOS devices.
    ///
    /// Defaults to [`Brightness::Light`].
    pub keyboard_appearance: Brightness,
    /// The configuration to use for autofill.
    pub autofill_configuration: AutofillConfiguration,
    /// Whether to enable that the IME update personalized data such as typing
    /// history and user dictionary data.
    ///
    /// Defaults to true.
    pub enable_ime_personalized_learning: bool,
    /// MIME types the field accepts for content insertion.
    pub allowed_mime_types: Vec<String>,
    /// Whether to enable that the engine sends text input updates to the
    /// framework as `TextEditingDelta`s or as one [`TextEditingValue`].
    ///
    /// Defaults to false.
    pub enable_delta_model: bool,
    /// List of the languages that the user is expected to use.
    ///
    /// Pass an empty list to express the intention that a specific hint should not be set.
    pub hint_locales: Vec<Locale>,
    /// Whether to enable inline predictive text.
    ///
    /// This feature is specific to iOS 17 and later. `None` leaves the platform default.
    pub enable_inline_prediction: Option<bool>,
}

impl TextInputConfiguration {
    /// Creates configuration information for a text input control.
    pub fn new() -> TextInputConfiguration {
        TextInputConfiguration {
            view_id: None,
            input_type: TextInputType::TEXT,
            read_only: false,
            obscure_text: false,
            autocorrect: true,
            smart_dashes_type: SmartDashesType::Enabled,
            smart_quotes_type: SmartQuotesType::Enabled,
            enable_suggestions: true,
            enable_interactive_selection: true,
            action_label: None,
            input_action: TextInputAction::Done,
            text_capitalization: TextCapitalization::None,
            keyboard_appearance: Brightness::Light,
            autofill_configuration: AutofillConfiguration::DISABLED,
            enable_ime_personalized_learning: true,
            allowed_mime_types: Vec::new(),
            enable_delta_model: false,
            hint_locales: Vec::new(),
            enable_inline_prediction: None,
        }
    }

    /// Dart `TextInputConfiguration(viewId:)`.
    pub fn view_id(mut self, view_id: i32) -> TextInputConfiguration {
        self.view_id = Some(view_id);
        self
    }

    /// Dart `TextInputConfiguration(inputType:)`.
    pub fn input_type(mut self, input_type: TextInputType) -> TextInputConfiguration {
        self.input_type = input_type;
        self
    }

    /// Dart `TextInputConfiguration(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> TextInputConfiguration {
        self.read_only = read_only;
        self
    }

    /// Dart `TextInputConfiguration(obscureText:)`.
    pub fn obscure_text(mut self, obscure_text: bool) -> TextInputConfiguration {
        self.obscure_text = obscure_text;
        self.smart_dashes_type = if obscure_text {
            SmartDashesType::Disabled
        } else {
            SmartDashesType::Enabled
        };
        self.smart_quotes_type = if obscure_text {
            SmartQuotesType::Disabled
        } else {
            SmartQuotesType::Enabled
        };
        self
    }

    /// Dart `TextInputConfiguration(autocorrect:)`.
    pub fn autocorrect(mut self, autocorrect: bool) -> TextInputConfiguration {
        self.autocorrect = autocorrect;
        self
    }

    /// Dart `TextInputConfiguration(smartDashesType:)`.
    pub fn smart_dashes_type(
        mut self,
        smart_dashes_type: SmartDashesType,
    ) -> TextInputConfiguration {
        self.smart_dashes_type = smart_dashes_type;
        self
    }

    /// Dart `TextInputConfiguration(smartQuotesType:)`.
    pub fn smart_quotes_type(
        mut self,
        smart_quotes_type: SmartQuotesType,
    ) -> TextInputConfiguration {
        self.smart_quotes_type = smart_quotes_type;
        self
    }

    /// Dart `TextInputConfiguration(enableSuggestions:)`.
    pub fn enable_suggestions(mut self, enable_suggestions: bool) -> TextInputConfiguration {
        self.enable_suggestions = enable_suggestions;
        self
    }

    /// Dart `TextInputConfiguration(enableInteractiveSelection:)`.
    pub fn enable_interactive_selection(
        mut self,
        enable_interactive_selection: bool,
    ) -> TextInputConfiguration {
        self.enable_interactive_selection = enable_interactive_selection;
        self
    }

    /// Dart `TextInputConfiguration(actionLabel:)`.
    pub fn action_label(mut self, action_label: impl Into<String>) -> TextInputConfiguration {
        self.action_label = Some(action_label.into());
        self
    }

    /// Dart `TextInputConfiguration(inputAction:)`.
    pub fn input_action(mut self, input_action: TextInputAction) -> TextInputConfiguration {
        self.input_action = input_action;
        self
    }

    /// Dart `TextInputConfiguration(keyboardAppearance:)`.
    pub fn keyboard_appearance(
        mut self,
        keyboard_appearance: Brightness,
    ) -> TextInputConfiguration {
        self.keyboard_appearance = keyboard_appearance;
        self
    }

    /// Dart `TextInputConfiguration(textCapitalization:)`.
    pub fn text_capitalization(
        mut self,
        text_capitalization: TextCapitalization,
    ) -> TextInputConfiguration {
        self.text_capitalization = text_capitalization;
        self
    }

    /// Dart `TextInputConfiguration(autofillConfiguration:)`.
    pub fn autofill_configuration(
        mut self,
        autofill_configuration: AutofillConfiguration,
    ) -> TextInputConfiguration {
        self.autofill_configuration = autofill_configuration;
        self
    }

    /// Dart `TextInputConfiguration(enableIMEPersonalizedLearning:)`.
    pub fn enable_ime_personalized_learning(
        mut self,
        enable_ime_personalized_learning: bool,
    ) -> TextInputConfiguration {
        self.enable_ime_personalized_learning = enable_ime_personalized_learning;
        self
    }

    /// Dart `TextInputConfiguration(allowedMimeTypes:)`.
    pub fn allowed_mime_types(mut self, allowed_mime_types: Vec<String>) -> TextInputConfiguration {
        self.allowed_mime_types = allowed_mime_types;
        self
    }

    /// Dart `TextInputConfiguration(enableDeltaModel:)`.
    pub fn enable_delta_model(mut self, enable_delta_model: bool) -> TextInputConfiguration {
        self.enable_delta_model = enable_delta_model;
        self
    }

    /// Dart `TextInputConfiguration(hintLocales:)`.
    pub fn hint_locales(mut self, hint_locales: Vec<Locale>) -> TextInputConfiguration {
        self.hint_locales = hint_locales;
        self
    }

    /// Dart `TextInputConfiguration(enableInlinePrediction:)`.
    pub fn enable_inline_prediction(
        mut self,
        enable_inline_prediction: bool,
    ) -> TextInputConfiguration {
        self.enable_inline_prediction = Some(enable_inline_prediction);
        self
    }

    /// Creates a copy of this [`TextInputConfiguration`] with the given fields
    /// replaced with new values.
    pub fn copy_with(&self) -> TextInputConfiguration {
        self.clone()
    }
}

impl Default for TextInputConfiguration {
    fn default() -> TextInputConfiguration {
        TextInputConfiguration::new()
    }
}

impl fmt::Display for TextInputConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![
            format!("inputType: {}", self.input_type),
            format!("readOnly: {}", self.read_only),
            format!("obscureText: {}", self.obscure_text),
            format!("autocorrect: {}", self.autocorrect),
            format!("smartDashesType: {:?}", self.smart_dashes_type),
            format!("smartQuotesType: {:?}", self.smart_quotes_type),
            format!("enableSuggestions: {}", self.enable_suggestions),
            format!(
                "enableInteractiveSelection: {}",
                self.enable_interactive_selection
            ),
            format!("inputAction: {:?}", self.input_action),
            format!("keyboardAppearance: {:?}", self.keyboard_appearance),
            format!("textCapitalization: {:?}", self.text_capitalization),
            format!("autofillConfiguration: {}", self.autofill_configuration),
            format!(
                "enableIMEPersonalizedLearning: {}",
                self.enable_ime_personalized_learning
            ),
            format!("allowedMimeTypes: {:?}", self.allowed_mime_types),
            format!("enableDeltaModel: {}", self.enable_delta_model),
        ];
        if let Some(view_id) = self.view_id {
            parts.insert(0, format!("viewId: {view_id}"));
        }
        if let Some(action_label) = &self.action_label {
            parts.insert(
                parts
                    .iter()
                    .position(|p| p.starts_with("inputAction:"))
                    .unwrap_or(parts.len()),
                format!("actionLabel: {action_label}"),
            );
        }
        if !self.hint_locales.is_empty() {
            parts.push(format!("hintLocales: {:?}", self.hint_locales));
        }
        if let Some(enable_inline_prediction) = self.enable_inline_prediction {
            parts.push(format!(
                "enableInlinePrediction: {enable_inline_prediction}"
            ));
        }
        write!(f, "TextInputConfiguration({})", parts.join(", "))
    }
}

/// The state of a "floating cursor" drag on an iOS soft keyboard.
///
/// The "floating cursor" cursor-positioning mode is an iOS feature used to
/// precisely position the caret in some editable text using certain touch
/// gestures. As an example, when the user long-presses the spacebar on the iOS
/// virtual keyboard, iOS enters floating cursor mode where the whole keyboard
/// becomes a trackpad. In this mode, there are two visible cursors. One, the
/// floating cursor, hovers over the text, following the user's horizontal
/// movements exactly and snapping to lines vertically. The other, the
/// placeholder cursor, is a "shadow" that also snaps to the actual location
/// where the cursor will go horizontally when the user releases the trackpad.
///
/// The floating cursor renders over the text field, while the placeholder
/// cursor is a faint shadow of the cursor rendered in the text field in the
/// location between characters where the cursor will drop into when released.
/// The placeholder cursor is a faint vertical bar, while the floating cursor
/// has the same appearance as a normal cursor (a blue vertical bar).
///
/// This feature works out-of-the-box with Flutter. Support is built into
/// `EditableText`.
///
/// See also:
///
///  * `EditableText.backgroundCursorColor`, which configures the color of the
///    placeholder cursor while the floating cursor is being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatingCursorDragState {
    /// A user has just activated a floating cursor by long pressing on the
    /// spacebar.
    Start,

    /// A user is dragging a floating cursor.
    Update,

    /// A user has lifted their finger off the screen after using a floating
    /// cursor.
    End,
}

/// The current state and position of the floating cursor.
///
/// See also:
///
///  * [`FloatingCursorDragState`], which explains the floating cursor feature in
///    detail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawFloatingCursorPoint {
    /// The raw position of the floating cursor as determined by the iOS sdk.
    pub offset: Option<Offset>,
    /// Represents the starting location when initiating a floating cursor via long press.
    /// This is a tuple where the first item is the local offset and the second item is the new caret position.
    /// This is only non-null when a floating cursor is started.
    pub start_location: Option<(Offset, TextPosition)>,
    /// The state of the floating cursor.
    pub state: FloatingCursorDragState,
}

impl RawFloatingCursorPoint {
    /// Creates information for setting the position and state of a floating
    /// cursor.
    ///
    /// [`state`](Self::state) must not be null and [`offset`](Self::offset) must not be null if the state is
    /// [`FloatingCursorDragState::Update`].
    pub fn new(state: FloatingCursorDragState) -> RawFloatingCursorPoint {
        let point = RawFloatingCursorPoint {
            offset: None,
            start_location: None,
            state,
        };
        if state == FloatingCursorDragState::Update {
            point
        } else {
            point.assemble()
        }
    }

    /// Dart `RawFloatingCursorPoint(offset:)`.
    pub fn offset(mut self, offset: Offset) -> RawFloatingCursorPoint {
        self.offset = Some(offset);
        self.assemble()
    }

    /// Dart `RawFloatingCursorPoint(startLocation:)`.
    pub fn start_location(
        mut self,
        start_location: (Offset, TextPosition),
    ) -> RawFloatingCursorPoint {
        self.start_location = Some(start_location);
        self
    }

    fn assemble(self) -> RawFloatingCursorPoint {
        debug_assert!(
            self.state != FloatingCursorDragState::Update || self.offset.is_some(),
            "offset must not be null if the state is Update"
        );
        self
    }
}

/// The current text, selection, and composing state for editing a run of text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextEditingValue {
    /// The current text being edited.
    pub text: String,
    /// The range of text that is currently selected.
    ///
    /// When [`selection`](Self::selection) is a [`TextSelection`] that has the same non-negative
    /// `baseOffset` and `extentOffset`, the [`selection`](Self::selection) property represents the
    /// caret position.
    ///
    /// If the current [`selection`](Self::selection) has a negative `baseOffset` or `extentOffset`,
    /// then the text currently does not have a selection or a caret location, and
    /// most text editing operations that rely on the current selection (for
    /// instance, insert a character at the caret location) will do nothing.
    pub selection: TextSelection,
    /// The range of text that is still being composed.
    ///
    /// Composing regions are created by input methods (IMEs) to indicate the text
    /// within a certain range is provisional. For instance, the Android Gboard
    /// app's English keyboard puts the current word under the caret into a
    /// composing region to indicate the word is subject to autocorrect or
    /// prediction changes.
    ///
    /// Composing regions can also be used for performing multistage input, which
    /// is typically used by IMEs designed for phonetic keyboard to enter
    /// ideographic symbols. As an example, many CJK keyboards require the user to
    /// enter a Latin alphabet sequence and then convert it to CJK characters. On
    /// iOS, the default software keyboards do not have a dedicated view to show
    /// the unfinished Latin sequence, so it's displayed directly in the text
    /// field, inside of a composing region.
    ///
    /// On iOS 17 and later, the composing region can also be used
    /// to display inline text predictions. The user can accept the
    /// predicted text by tapping the Space bar.
    ///
    /// The composing region should typically only be changed by the IME, or the
    /// user via interacting with the IME.
    ///
    /// If the range represented by this property is [`TextRange::EMPTY`], then the
    /// text is not currently being composed.
    pub composing: TextRange,
}

impl TextEditingValue {
    /// A value that corresponds to the empty string with no selection and no composing range.
    pub const EMPTY: TextEditingValue = TextEditingValue {
        text: String::new(),
        selection: TextSelection::collapsed(-1, TextAffinity::Downstream),
        composing: TextRange::EMPTY,
    };

    /// Creates information for editing a run of text.
    ///
    /// The selection and composing range must be within the text. This is not
    /// checked during construction, and must be guaranteed by the caller.
    ///
    /// The default value of [`selection`](Self::selection) is `TextSelection::collapsed(-1, …)`.
    /// This indicates that there is no selection at all.
    pub fn new() -> TextEditingValue {
        TextEditingValue::EMPTY
    }

    /// Dart `TextEditingValue(text:)`.
    pub fn text(mut self, text: impl Into<String>) -> TextEditingValue {
        self.text = text.into();
        self
    }

    /// Dart `TextEditingValue(selection:)`.
    pub fn selection(mut self, selection: TextSelection) -> TextEditingValue {
        self.selection = selection;
        self
    }

    /// Dart `TextEditingValue(composing:)`.
    pub fn composing(mut self, composing: TextRange) -> TextEditingValue {
        self.composing = composing;
        self
    }

    /// Creates a copy of this value but with the given fields replaced with the new values.
    pub fn copy_with(&self) -> TextEditingValue {
        self.clone()
    }

    /// Whether the [`composing`](Self::composing) range is a valid range within [`text`](Self::text).
    ///
    /// Returns true if and only if the [`composing`](Self::composing) range is normalized, its start
    /// is greater than or equal to 0, and its end is less than or equal to
    /// [`text`](Self::text)'s length.
    ///
    /// If this property is false while the [`composing`](Self::composing) range's `isValid` is true,
    /// it usually indicates the current [`composing`](Self::composing) range is invalid because of a
    /// programming error.
    pub fn is_composing_range_valid(&self) -> bool {
        self.composing.is_valid()
            && self.composing.is_normalized()
            && self.composing.end <= utf16_len(&self.text)
    }

    /// Returns a new [`TextEditingValue`], which is this [`TextEditingValue`] with
    /// its [`text`](Self::text) partially replaced by the `replacement_string`.
    ///
    /// The `replacement_range` parameter specifies the range of the
    /// [`text`](Self::text) that needs to be replaced.
    ///
    /// The `replacement_string` parameter specifies the string to replace the
    /// given range of text with.
    ///
    /// This method also adjusts the selection range and the composing range of the
    /// resulting [`TextEditingValue`], such that they point to the same substrings
    /// as the corresponding ranges in the original [`TextEditingValue`]. For
    /// example, if the original [`TextEditingValue`] is "Hello world" with the word
    /// "world" selected, replacing "Hello" with a different string using this
    /// method will not change the selected word.
    ///
    /// This method does nothing if the given `replacement_range` is not
    /// [`TextRange::is_valid`].
    pub fn replaced(
        &self,
        replacement_range: impl Into<TextRange>,
        replacement_string: &str,
    ) -> TextEditingValue {
        let replacement_range = replacement_range.into();
        if !replacement_range.is_valid() {
            return self.clone();
        }
        debug_assert!(replacement_range.is_normalized());
        let new_text = format!(
            "{}{}{}",
            replacement_range.text_before(&self.text),
            replacement_string,
            replacement_range.text_after(&self.text),
        );
        if replacement_range.end - replacement_range.start == utf16_len(replacement_string) {
            return self.copy_with().text(new_text);
        }

        let replacement_len = utf16_len(replacement_string);
        let adjust_index = |original_index: i32| {
            // The length added by adding the replacementString.
            let replaced_length = if original_index <= replacement_range.start
                && original_index < replacement_range.end
            {
                0
            } else {
                replacement_len
            };
            // The length removed by removing the replacementRange.
            let removed_length = original_index
                .clamp(replacement_range.start, replacement_range.end)
                - replacement_range.start;
            original_index + replaced_length - removed_length
        };

        let adjusted_selection = TextSelection::new(
            adjust_index(self.selection.base_offset),
            adjust_index(self.selection.extent_offset),
        );
        let adjusted_composing = TextRange::new(
            adjust_index(self.composing.start),
            adjust_index(self.composing.end),
        );
        debug_assert!(text_range_is_valid(adjusted_selection.range(), &new_text));
        debug_assert!(text_range_is_valid(adjusted_composing, &new_text));
        TextEditingValue::new()
            .text(new_text)
            .selection(adjusted_selection)
            .composing(adjusted_composing)
    }
}

impl Default for TextEditingValue {
    fn default() -> TextEditingValue {
        TextEditingValue::new()
    }
}

impl fmt::Display for TextEditingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TextEditingValue(text: \u{2524}{}\u{251C}, selection: {}, composing: {:?})",
            self.text, self.selection, self.composing
        )
    }
}

/// Dart `String.length`: UTF-16 code units.
fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// Verify that the given range is within the text.
///
/// The verification can't be perform during the constructor of
/// [`TextEditingValue`], which are `const` and are allowed to retrieve
/// properties of [`TextRange`]s. [`TextEditingValue`] should perform this
/// wherever it is building other values (such as toJson) or is built in a
/// non-const way (such as fromJson).
fn text_range_is_valid(range: TextRange, text: &str) -> bool {
    if range.start == -1 && range.end == -1 {
        return true;
    }
    let len = utf16_len(text);
    debug_assert!(
        range.start >= 0 && range.start <= len,
        "Range start {} is out of text of length {len}",
        range.start
    );
    debug_assert!(
        range.end >= 0 && range.end <= len,
        "Range end {} is out of text of length {len}",
        range.end
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TEXT: &str = "From a false proposition, anything follows.";

    #[test]
    fn replaced_deletes_a_selection() {
        let selection = TextSelection::new(5, 13);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(selection, ""),
            TextEditingValue::new()
                .text("From proposition, anything follows.")
                .selection(TextSelection::collapsed(5, TextAffinity::Downstream))
        );
    }

    #[test]
    fn replaced_deletes_a_reversed_selection() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(selection, ""),
            TextEditingValue::new()
                .text("From proposition, anything follows.")
                .selection(TextSelection::collapsed(5, TextAffinity::Downstream))
        );
    }

    #[test]
    fn replaced_inserts_at_the_caret() {
        let selection = TextSelection::collapsed(5, TextAffinity::Downstream);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(selection, "AA"),
            TextEditingValue::new()
                .text("From AAa false proposition, anything follows.")
                .selection(TextSelection::collapsed(7, TextAffinity::Downstream))
        );
    }

    #[test]
    fn replaced_before_selection_shifts_the_range() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(TextRange::new(4, 5), "AA"),
            TextEditingValue::new()
                .text("FromAAa false proposition, anything follows.")
                .selection(TextSelection::new(14, 6))
        );
    }

    #[test]
    fn replaced_after_selection_leaves_the_range() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(TextRange::new(13, 14), "AA"),
            TextEditingValue::new()
                .text("From a false AAroposition, anything follows.")
                .selection(selection)
        );
    }

    #[test]
    fn replaced_inside_selection_start_boundary() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(TextRange::new(5, 6), "AA"),
            TextEditingValue::new()
                .text("From AA false proposition, anything follows.")
                .selection(TextSelection::new(14, 5))
        );
    }

    #[test]
    fn replaced_inside_selection_end_boundary() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(TextRange::new(12, 13), "AA"),
            TextEditingValue::new()
                .text("From a falseAAproposition, anything follows.")
                .selection(TextSelection::new(14, 5))
        );
    }

    #[test]
    fn replaced_deletes_after_selection() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(TextRange::new(13, 14), ""),
            TextEditingValue::new()
                .text("From a false roposition, anything follows.")
                .selection(selection)
        );
    }

    #[test]
    fn replaced_deletes_inside_selection_start_boundary() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(TextRange::new(5, 6), ""),
            TextEditingValue::new()
                .text("From  false proposition, anything follows.")
                .selection(TextSelection::new(12, 5))
        );
    }

    #[test]
    fn replaced_deletes_inside_selection_end_boundary() {
        let selection = TextSelection::new(13, 5);
        assert_eq!(
            TextEditingValue::new()
                .text(TEST_TEXT)
                .selection(selection)
                .replaced(TextRange::new(12, 13), ""),
            TextEditingValue::new()
                .text("From a falseproposition, anything follows.")
                .selection(TextSelection::new(12, 5))
        );
    }

    #[test]
    fn empty_prints_like_dart() {
        assert_eq!(
            TextEditingValue::EMPTY.to_string(),
            format!(
                "TextEditingValue(text: \u{2524}\u{251C}, selection: {}, composing: {:?})",
                TextSelection::collapsed(-1, TextAffinity::Downstream),
                TextRange::EMPTY
            )
        );
    }

    #[test]
    fn number_with_options_defaults_match_dart() {
        assert_eq!(TextInputType::NUMBER.signed, Some(false));
        assert_eq!(
            TextInputType::number_with_options()
                .signed(true)
                .decimal(true),
            TextInputType {
                index: 2,
                signed: Some(true),
                decimal: Some(true),
                password: Some(false),
            }
        );
    }

    #[test]
    fn obscure_text_disables_smart_punctuation_unless_overridden() {
        let obscured = TextInputConfiguration::new().obscure_text(true);
        assert!(obscured.obscure_text);
        assert_eq!(obscured.smart_dashes_type, SmartDashesType::Disabled);
        assert_eq!(obscured.smart_quotes_type, SmartQuotesType::Disabled);

        let restored = obscured.smart_dashes_type(SmartDashesType::Enabled);
        assert_eq!(restored.smart_dashes_type, SmartDashesType::Enabled);
    }

    #[test]
    fn autofill_disabled_is_the_configuration_default() {
        let configuration = TextInputConfiguration::new();
        assert!(!configuration.autofill_configuration.enabled);
        assert_eq!(
            AutofillConfiguration::new("id", vec!["email".into()], TextEditingValue::EMPTY)
                .hint_text("email")
                .hint_text,
            Some("email".into())
        );
    }
}

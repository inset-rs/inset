//! Flutter counterpart: `services/autofill.dart`.
//!
//! `AutofillConfiguration` lives in `inset-embedder` (re-exported from
//! `text_input.rs`) because `View` names `TextInputConfiguration`.

use inset_foundation::{App, Handle, HandleId};

use crate::text_input::{
    AnyTextInputClient, TextEditingValue, TextInput, TextInputConfiguration, TextInputConnection,
};

/// A collection of commonly used autofill hint strings on different platforms.
///
/// Each hint is pre-defined on at least one supported platform. See their
/// documentation for their availability on each platform, and the platform
/// values each autofill hint corresponds to.
pub struct AutofillHints;

impl AutofillHints {
    /// The input field expects an address locality (city/town).
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_ADDRESS_LOCALITY](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_ADDRESS_LOCALITY).
    /// * iOS: [addressCity](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * Otherwise, the hint string will be used as-is.
    pub const ADDRESS_CITY: &str = "addressCity";

    /// The input field expects a city name combined with a state name.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [addressCityAndState](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * Otherwise, the hint string will be used as-is.
    pub const ADDRESS_CITY_AND_STATE: &str = "addressCityAndState";

    /// The input field expects a region/state.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_ADDRESS_REGION](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_ADDRESS_REGION).
    /// * iOS: [addressState](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * Otherwise, the hint string will be used as-is.
    pub const ADDRESS_STATE: &str = "addressState";

    /// The input field expects a person's full birth date.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_BIRTH_DATE_FULL](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_BIRTH_DATE_FULL).
    /// * web: ["bday"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const BIRTHDAY: &str = "birthday";

    /// The input field expects a person's birth day(of the month).
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_BIRTH_DATE_DAY](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_BIRTH_DATE_DAY).
    /// * web: ["bday-day"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const BIRTHDAY_DAY: &str = "birthdayDay";

    /// The input field expects a person's birth month.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_BIRTH_DATE_MONTH](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_BIRTH_DATE_MONTH).
    /// * web: ["bday-month"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const BIRTHDAY_MONTH: &str = "birthdayMonth";

    /// The input field expects a person's birth year.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_BIRTH_DATE_YEAR](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_BIRTH_DATE_YEAR).
    /// * web: ["bday-year"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const BIRTHDAY_YEAR: &str = "birthdayYear";

    /// The input field expects an
    /// [ISO 3166-1-alpha-2](https://www.iso.org/standard/63545.html) country code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["country"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const COUNTRY_CODE: &str = "countryCode";

    /// The input field expects a country name.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_ADDRESS_COUNTRY](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_ADDRESS_COUNTRY).
    /// * iOS: [countryName](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["country-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const COUNTRY_NAME: &str = "countryName";

    /// The input field expects a credit card expiration date.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_CREDIT_CARD_NUMBER](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_CREDIT_CARD_NUMBER).
    /// * web: ["cc-exp"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_EXPIRATION_DATE: &str = "creditCardExpirationDate";

    /// The input field expects a credit card expiration day.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_CREDIT_CARD_EXPIRATION_DAY](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_CREDIT_CARD_EXPIRATION_DAY).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_EXPIRATION_DAY: &str = "creditCardExpirationDay";

    /// The input field expects a credit card expiration month.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_CREDIT_CARD_EXPIRATION_MONTH](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_CREDIT_CARD_EXPIRATION_MONTH).
    /// * web: ["cc-exp-month"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_EXPIRATION_MONTH: &str = "creditCardExpirationMonth";

    /// The input field expects a credit card expiration year.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_CREDIT_CARD_EXPIRATION_YEAR](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_CREDIT_CARD_EXPIRATION_YEAR).
    /// * web: ["cc-exp-year"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_EXPIRATION_YEAR: &str = "creditCardExpirationYear";

    /// The input field expects the holder's last/family name as given on a credit
    /// card.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["cc-family-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_FAMILY_NAME: &str = "creditCardFamilyName";

    /// The input field expects the holder's first/given name as given on a credit
    /// card.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["cc-given-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_GIVEN_NAME: &str = "creditCardGivenName";

    /// The input field expects the holder's middle name as given on a credit
    /// card.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["cc-additional-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_MIDDLE_NAME: &str = "creditCardMiddleName";

    /// The input field expects the holder's full name as given on a credit card.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["cc-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_NAME: &str = "creditCardName";

    /// The input field expects a credit card number.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_CREDIT_CARD_NUMBER](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_CREDIT_CARD_NUMBER).
    /// * iOS: [creditCardNumber](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["cc-number"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_NUMBER: &str = "creditCardNumber";

    /// The input field expects a credit card security code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_CREDIT_CARD_SECURITY_CODE](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_CREDIT_CARD_SECURITY_CODE).
    /// * web: ["cc-csc"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_SECURITY_CODE: &str = "creditCardSecurityCode";

    /// The input field expects the type of a credit card, for example "Visa".
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["cc-type"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const CREDIT_CARD_TYPE: &str = "creditCardType";

    /// The input field expects an email address.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_EMAIL_ADDRESS](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_EMAIL_ADDRESS).
    /// * iOS: [emailAddress](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["email"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const EMAIL: &str = "email";

    /// The input field expects a person's last/family name.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PERSON_NAME_FAMILY](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PERSON_NAME_FAMILY).
    /// * iOS: [familyName](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["family-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const FAMILY_NAME: &str = "familyName";

    /// The input field expects a street address that fully identifies a location.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_ADDRESS_STREET_ADDRESS](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_ADDRESS_STREET_ADDRESS).
    /// * iOS: [fullStreetAddress](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["street-address"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const FULL_STREET_ADDRESS: &str = "fullStreetAddress";

    /// The input field expects a gender.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_GENDER](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_GENDER).
    /// * web: ["sex"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const GENDER: &str = "gender";

    /// The input field expects a person's first/given name.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PERSON_NAME_GIVEN](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PERSON_NAME_GIVEN).
    /// * iOS: [givenName](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["given-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const GIVEN_NAME: &str = "givenName";

    /// The input field expects a URL representing an instant messaging protocol
    /// endpoint.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["impp"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const IMPP: &str = "impp";

    /// The input field expects a job title.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [jobTitle](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["organization-title"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const JOB_TITLE: &str = "jobTitle";

    /// The input field expects the preferred language of the user.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["language"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const LANGUAGE: &str = "language";

    /// The input field expects a location, such as a point of interest, an
    /// address,or another way to identify a location.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [location](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * Otherwise, the hint string will be used as-is.
    pub const LOCATION: &str = "location";

    /// The input field expects a person's middle initial.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PERSON_NAME_MIDDLE_INITIAL](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PERSON_NAME_MIDDLE_INITIAL).
    /// * Otherwise, the hint string will be used as-is.
    pub const MIDDLE_INITIAL: &str = "middleInitial";

    /// The input field expects a person's middle name.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PERSON_NAME_MIDDLE](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PERSON_NAME_MIDDLE).
    /// * iOS: [middleName](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["additional-name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const MIDDLE_NAME: &str = "middleName";

    /// The input field expects a person's full name.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PERSON_NAME](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PERSON_NAME).
    /// * iOS: [name](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["name"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const NAME: &str = "name";

    /// The input field expects a person's name prefix or title, such as "Dr.".
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PERSON_NAME_PREFIX](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PERSON_NAME_PREFIX).
    /// * iOS: [namePrefix](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["honorific-prefix"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const NAME_PREFIX: &str = "namePrefix";

    /// The input field expects a person's name suffix, such as "Jr.".
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PERSON_NAME_SUFFIX](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PERSON_NAME_SUFFIX).
    /// * iOS: [nameSuffix](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["honorific-suffix"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const NAME_SUFFIX: &str = "nameSuffix";

    /// The input field expects a newly created password for save/update.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_NEW_PASSWORD](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_NEW_PASSWORD).
    /// * iOS: [newPassword](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["new-password"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const NEW_PASSWORD: &str = "newPassword";

    /// The input field expects a newly created username for save/update.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_NEW_USERNAME](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_NEW_USERNAME).
    /// * Otherwise, the hint string will be used as-is.
    pub const NEW_USERNAME: &str = "newUsername";

    /// The input field expects a nickname.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [nickname](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["nickname"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const NICKNAME: &str = "nickname";

    /// The input field expects a SMS one-time code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_SMS_OTP](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_SMS_OTP).
    /// * iOS: [oneTimeCode](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["one-time-code"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const ONE_TIME_CODE: &str = "oneTimeCode";

    /// The input field expects an email one-time code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_EMAIL_OTP](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_EMAIL_OTP).
    /// * Otherwise, the hint string will be used as-is.
    pub const EMAIL_OTP_CODE: &str = "emailOTPCode";

    /// The input field expects an organization name corresponding to the person,
    /// address, or contact information in the other fields associated with this
    /// field.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [organizationName](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["organization"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const ORGANIZATION_NAME: &str = "organizationName";

    /// The input field expects a password.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PASSWORD](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PASSWORD).
    /// * iOS: [password](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["current-password"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const PASSWORD: &str = "password";

    /// The input field expects a photograph, icon, or other image corresponding
    /// to the company, person, address, or contact information in the other
    /// fields associated with this field.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["photo"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const PHOTO: &str = "photo";

    /// The input field expects a postal address.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_ADDRESS](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_ADDRESS).
    /// * Otherwise, the hint string will be used as-is.
    pub const POSTAL_ADDRESS: &str = "postalAddress";

    /// The input field expects an auxiliary address details.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_ADDRESS_EXTENDED_ADDRESS](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_ADDRESS_EXTENDED_ADDRESS).
    /// * Otherwise, the hint string will be used as-is.
    pub const POSTAL_ADDRESS_EXTENDED: &str = "postalAddressExtended";

    /// The input field expects an extended ZIP/POSTAL code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_ADDRESS_EXTENDED_POSTAL_CODE](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_ADDRESS_EXTENDED_POSTAL_CODE).
    /// * Otherwise, the hint string will be used as-is.
    pub const POSTAL_ADDRESS_EXTENDED_POSTAL_CODE: &str = "postalAddressExtendedPostalCode";

    /// The input field expects a postal code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_POSTAL_CODE](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_POSTAL_CODE).
    /// * iOS: [postalCode](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["postal-code"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const POSTAL_CODE: &str = "postalCode";

    /// The first administrative level in the address. This is typically the
    /// province in which the address is located. In the United States, this would
    /// be the state. In Switzerland, the canton. In the United Kingdom, the post
    /// town.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["address-level1"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const STREET_ADDRESS_LEVEL1: &str = "streetAddressLevel1";

    /// The second administrative level, in addresses with at least two of them.
    /// In countries with two administrative levels, this would typically be the
    /// city, town, village, or other locality in which the address is located.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["address-level2"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const STREET_ADDRESS_LEVEL2: &str = "streetAddressLevel2";

    /// The third administrative level, in addresses with at least three
    /// administrative levels.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["address-level3"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const STREET_ADDRESS_LEVEL3: &str = "streetAddressLevel3";

    /// The finest-grained administrative level, in addresses which have four
    /// levels.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["address-level4"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const STREET_ADDRESS_LEVEL4: &str = "streetAddressLevel4";

    /// The input field expects the first line of a street address.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [streetAddressLine1](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["address-line1"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const STREET_ADDRESS_LINE1: &str = "streetAddressLine1";

    /// The input field expects the second line of a street address.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [streetAddressLine2](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    ///   As of iOS 14.2 this hint does not trigger autofill.
    /// * web: ["address-line2"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const STREET_ADDRESS_LINE2: &str = "streetAddressLine2";

    /// The input field expects the third line of a street address.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["address-line3"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const STREET_ADDRESS_LINE3: &str = "streetAddressLine3";

    /// The input field expects a sublocality.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [sublocality](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * Otherwise, the hint string will be used as-is.
    pub const SUBLOCALITY: &str = "sublocality";

    /// The input field expects a telephone number.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PHONE_NUMBER](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PHONE_NUMBER).
    /// * iOS: [telephoneNumber](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["tel"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER: &str = "telephoneNumber";

    /// The input field expects a phone number's area code, with a country
    /// -internal prefix applied if applicable.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["tel-area-code"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_AREA_CODE: &str = "telephoneNumberAreaCode";

    /// The input field expects a phone number's country code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PHONE_COUNTRY_CODE](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PHONE_COUNTRY_CODE).
    /// * web: ["tel-country-code"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_COUNTRY_CODE: &str = "telephoneNumberCountryCode";

    /// The input field expects the current device's phone number, usually for
    /// Sign Up / OTP flows.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PHONE_NUMBER_DEVICE](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PHONE_NUMBER_DEVICE).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_DEVICE: &str = "telephoneNumberDevice";

    /// The input field expects a phone number's internal extension code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["tel-extension"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_EXTENSION: &str = "telephoneNumberExtension";

    /// The input field expects a phone number without the country code and area
    /// code components.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["tel-local"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_LOCAL: &str = "telephoneNumberLocal";

    /// The input field expects the first part of the component of the telephone
    /// number that follows the area code, when that component is split into two
    /// components.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["tel-local-prefix"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_LOCAL_PREFIX: &str = "telephoneNumberLocalPrefix";

    /// The input field expects the second part of the component of the telephone
    /// number that follows the area code, when that component is split into two
    /// components.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["tel-local-suffix"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_LOCAL_SUFFIX: &str = "telephoneNumberLocalSuffix";

    /// The input field expects a phone number without country code.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_PHONE_NATIONAL](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_PHONE_NATIONAL).
    /// * web: ["tel-national"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TELEPHONE_NUMBER_NATIONAL: &str = "telephoneNumberNational";

    /// The amount that the user would like for the transaction (e.g. when
    /// entering a bid or sale price).
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["transaction-amount"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TRANSACTION_AMOUNT: &str = "transactionAmount";

    /// The currency that the user would prefer the transaction to use, in [ISO
    /// 4217 currency code](https://www.iso.org/iso-4217-currency-codes.html).
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * web: ["transaction-currency"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const TRANSACTION_CURRENCY: &str = "transactionCurrency";

    /// The input field expects a URL.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * iOS: [URL](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["url"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const URL: &str = "url";

    /// The input field expects a username or an account name.
    ///
    /// This hint will be translated to the below values on different platforms:
    ///
    /// * Android: [AUTOFILL_HINT_USERNAME](https://developer.android.com/reference/androidx/autofill/HintConstants#AUTOFILL_HINT_USERNAME).
    /// * iOS: [username](https://developer.apple.com/documentation/uikit/uitextcontenttype).
    /// * web: ["username"](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofilling-form-controls:-the-autocomplete-attribute).
    /// * Otherwise, the hint string will be used as-is.
    pub const USERNAME: &str = "username";
}

/// An object that represents an autofillable input field in the autofill workflow.
///
/// An [`AutofillClient`] provides autofill-related information of the input field
/// it represents to the platform, and consumes autofill inputs from the platform.
pub trait AutofillClient: Sized + 'static {
    /// The unique identifier of this [`AutofillClient`].
    ///
    /// The identifier must not be changed.
    fn autofill_id(self: Handle<Self>, app: &App) -> String;

    /// The [`TextInputConfiguration`] that describes this [`AutofillClient`].
    ///
    /// In order to participate in autofill, its
    /// [`TextInputConfiguration::autofill_configuration`] must not be
    /// [`AutofillConfiguration::DISABLED`](crate::AutofillConfiguration::DISABLED).
    fn text_input_configuration(self: Handle<Self>, app: &App) -> TextInputConfiguration;

    /// Requests this [`AutofillClient`] update its [`TextEditingValue`] to the given
    /// value.
    fn autofill(self: Handle<Self>, app: &mut App, new_editing_value: TextEditingValue);

    /// This client as the erased [`AnyAutofillClient`].
    fn as_autofill_client(self: Handle<Self>) -> AnyAutofillClient {
        AnyAutofillClient {
            id: self.id(),
            vtable: const { &AutofillClientVTable::of::<Self>() },
        }
    }
}

struct AutofillClientVTable {
    autofill_id: fn(&App, HandleId) -> String,
    text_input_configuration: fn(&App, HandleId) -> TextInputConfiguration,
    autofill: fn(&mut App, HandleId, TextEditingValue),
}

fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl AutofillClientVTable {
    const fn of<C: AutofillClient>() -> AutofillClientVTable {
        AutofillClientVTable {
            autofill_id: |app, id| C::autofill_id(resolve(id), app),
            text_input_configuration: |app, id| C::text_input_configuration(resolve(id), app),
            autofill: |app, id, value| C::autofill(resolve(id), app, value),
        }
    }
}

/// Erased [`AutofillClient`]: one identity and a static vtable.
#[derive(Clone, Copy)]
pub struct AnyAutofillClient {
    id: HandleId,
    vtable: &'static AutofillClientVTable,
}

impl AnyAutofillClient {
    pub fn autofill_id(self, app: &App) -> String {
        (self.vtable.autofill_id)(app, self.id)
    }

    pub fn text_input_configuration(self, app: &App) -> TextInputConfiguration {
        (self.vtable.text_input_configuration)(app, self.id)
    }

    pub fn autofill(self, app: &mut App, new_editing_value: TextEditingValue) {
        (self.vtable.autofill)(app, self.id, new_editing_value);
    }
}

impl PartialEq for AnyAutofillClient {
    fn eq(&self, other: &AnyAutofillClient) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyAutofillClient {}

/// An ordered group within which [`AutofillClient`]s are logically connected.
///
/// [`AutofillClient`]s within the same [`AutofillScope`] are isolated from other
/// input fields during autofill. That is, when an autofillable `TextInputClient`
/// gains focus, only the [`AutofillClient`]s within the same [`AutofillScope`] will
/// be visible to the autofill service, in the same order as they appear in
/// [`AutofillScope::autofill_clients`].
///
/// [`AutofillScope`] also allows `TextInput` to redirect autofill values from the
/// platform to the [`AutofillClient`] with the given identifier, by calling
/// [`AutofillScope::get_autofill_client`].
///
/// An [`AutofillClient`] that's not tied to any [`AutofillScope`] will only
/// participate in autofill if the autofill is directly triggered by its own
/// `TextInputClient`.
pub trait AutofillScope: Sized + 'static {
    /// Gets the [`AutofillClient`] associated with the given `autofill_id`, in
    /// this [`AutofillScope`].
    ///
    /// Returns [`None`] if there's no matching [`AutofillClient`].
    fn get_autofill_client(
        self: Handle<Self>,
        app: &App,
        autofill_id: &str,
    ) -> Option<AnyAutofillClient>;

    /// The collection of [`AutofillClient`]s currently tied to this [`AutofillScope`].
    ///
    /// Every [`AutofillClient`] in this list must have autofill enabled (i.e. its
    /// [`AutofillClient::text_input_configuration`] must have a non-disabled
    /// [`crate::AutofillConfiguration`].)
    fn autofill_clients(self: Handle<Self>, app: &App) -> Vec<AnyAutofillClient>;

    /// Allows a `TextInputClient` to attach to this scope. This method should be
    /// called in lieu of [`TextInput::attach`], when the `TextInputClient` wishes to
    /// participate in autofill.
    ///
    /// Flutter `AutofillScopeMixin.attach`.
    fn attach(
        self: Handle<Self>,
        app: &mut App,
        trigger: AnyTextInputClient,
        configuration: TextInputConfiguration,
    ) -> Handle<TextInputConnection> {
        debug_assert!(
            !self.autofill_clients(app).iter().any(|client| {
                !client
                    .text_input_configuration(app)
                    .autofill_configuration
                    .enabled
            }),
            "Every client in AutofillScope.autofillClients must enable autofill"
        );
        TextInput::attach(app, trigger, configuration)
    }
}

/// A partial implementation of [`AutofillScope`].
///
/// The mixin provides a default implementation for [`AutofillScope::attach`].
pub trait AutofillScopeMixin: AutofillScope {}

impl<T: AutofillScope> AutofillScopeMixin for T {}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;

    use super::*;
    use crate::text_input::TextInputClient;

    #[test]
    fn hint_strings_match_dart() {
        assert_eq!(AutofillHints::EMAIL, "email");
        assert_eq!(AutofillHints::ONE_TIME_CODE, "oneTimeCode");
        assert_eq!(AutofillHints::EMAIL_OTP_CODE, "emailOTPCode");
    }

    #[derive(Default)]
    struct RecordingField {
        autofill_id: String,
        value: TextEditingValue,
        filled: Option<TextEditingValue>,
    }

    impl AutofillClient for RecordingField {
        fn autofill_id(self: Handle<Self>, app: &App) -> String {
            app.get(self).autofill_id.clone()
        }

        fn text_input_configuration(self: Handle<Self>, app: &App) -> TextInputConfiguration {
            TextInputConfiguration::new().autofill_configuration(crate::AutofillConfiguration::new(
                app.get(self).autofill_id.clone(),
                vec![AutofillHints::EMAIL.to_owned()],
                app.get(self).value.clone(),
            ))
        }

        fn autofill(self: Handle<Self>, app: &mut App, new_editing_value: TextEditingValue) {
            app.get_mut(self).filled = Some(new_editing_value);
        }
    }

    impl TextInputClient for RecordingField {
        fn current_text_editing_value(self: Handle<Self>, app: &App) -> Option<TextEditingValue> {
            Some(app.get(self).value.clone())
        }

        fn update_editing_value(self: Handle<Self>, app: &mut App, value: TextEditingValue) {
            app.get_mut(self).value = value;
        }

        fn perform_action(self: Handle<Self>, _app: &mut App, _action: crate::TextInputAction) {}

        fn connection_closed(self: Handle<Self>, _app: &mut App) {}
    }

    struct RecordingScope {
        clients: Vec<AnyAutofillClient>,
    }

    impl AutofillScope for RecordingScope {
        fn get_autofill_client(
            self: Handle<Self>,
            app: &App,
            autofill_id: &str,
        ) -> Option<AnyAutofillClient> {
            self.autofill_clients(app)
                .into_iter()
                .find(|client| client.autofill_id(app) == autofill_id)
        }

        fn autofill_clients(self: Handle<Self>, app: &App) -> Vec<AnyAutofillClient> {
            app.get(self).clients.clone()
        }
    }

    #[test]
    fn scope_attach_uses_text_input_attach() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let field = app.create(RecordingField {
            autofill_id: "email".into(),
            ..RecordingField::default()
        });
        let scope = app.create(RecordingScope {
            clients: vec![field.as_autofill_client()],
        });
        let configuration = field.text_input_configuration(&app);
        let connection = scope.attach(&mut app, field.as_text_input_client(), configuration);
        assert!(connection.attached(&mut app));
        assert_eq!(
            scope
                .get_autofill_client(&app, "email")
                .map(|c| c.autofill_id(&app)),
            Some("email".into())
        );
        field
            .as_autofill_client()
            .autofill(&mut app, TextEditingValue::new().text("user@example.com"));
        assert_eq!(
            app.get(field).filled.as_ref().map(|v| v.text.as_str()),
            Some("user@example.com")
        );
    }
}

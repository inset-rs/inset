//! The publisher's settings the environment holds: signing identities, notarization credentials.
//! An empty variable is unset, as GitHub Actions and other CI pass a secret nobody set as an empty
//! string; reading it as a value would ask for a signing or a notarization with nothing to do it
//! with.

use std::ffi::OsString;

/// `name`'s value; `None` where it is unset or empty.
pub fn var_os(name: &str) -> Option<OsString> {
    std::env::var_os(name).and_then(set)
}

/// `name`'s value as text; `None` where it is unset, empty or not Unicode.
pub fn var(name: &str) -> Option<String> {
    var_os(name).and_then(|value| value.into_string().ok())
}

/// `value`, where it says something.
fn set(value: OsString) -> Option<OsString> {
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_variable_is_unset_and_a_value_is_kept() {
        assert_eq!(set(OsString::new()), None);
        assert_eq!(set("-".into()), Some("-".into()));
    }
}

//! The arguments a toast and its buttons carry, which the system hands back on a click, and the
//! click they tell: `inset-notification=<id>`, then `&action=<id>` for a button, then
//! `&input=<id>` for a reply action's box, each value percent-encoded so an id may hold any text.
//!
//! Arguments that do not start with `inset-notification=` are no toast of the embedder's, and
//! tell nothing.

use std::fmt::Write;

use inset_embedder::NotificationActivation;

const NOTIFICATION: &str = "inset-notification";
const ACTION: &str = "action";
const INPUT: &str = "input";

/// The arguments of a click on the notification `id` itself.
pub(crate) fn of_notification(id: &str) -> String {
    format!("{NOTIFICATION}={}", encoded(id))
}

/// The arguments of a click on the action `action` of the notification `id`, whose reply, for a
/// reply action, is in the box `input`.
pub(crate) fn of_action(id: &str, action: &str, input: Option<&str>) -> String {
    let mut arguments = format!("{}&{ACTION}={}", of_notification(id), encoded(action));
    if let Some(input) = input {
        let _ = write!(arguments, "&{INPUT}={}", encoded(input));
    }
    arguments
}

/// The click `arguments` tell, its reply read from the box they name through `typed`; `None`
/// for arguments of no toast of the embedder's.
pub(crate) fn activation(
    arguments: &str,
    typed: impl Fn(&str) -> Option<String>,
) -> Option<NotificationActivation> {
    let mut id = None;
    let mut action = None;
    let mut input = None;
    for pair in arguments.split('&') {
        let (key, value) = pair.split_once('=')?;
        let value = decoded(value)?;
        match key {
            NOTIFICATION => id = Some(value),
            ACTION => action = Some(value),
            INPUT => input = Some(value),
            _ => return None,
        }
    }
    let reply = input.map(|input| typed(&input).unwrap_or_default());
    Some(NotificationActivation {
        id: id?,
        action,
        reply,
    })
}

/// `text` with every byte but an unreserved one percent-encoded, as its UTF-8.
fn encoded(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// What `encoded` encoded; `None` for text it could not have made.
fn decoded(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        if byte == b'%' {
            let hex = after.get(..2)?;
            bytes.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            rest = &after[2..];
        } else {
            bytes.push(byte);
            rest = after;
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing_typed(_input: &str) -> Option<String> {
        None
    }

    #[test]
    fn a_click_on_the_notification_names_it_alone() {
        let arguments = of_notification("message 7 & more=é");
        assert_eq!(
            arguments,
            "inset-notification=message%207%20%26%20more%3D%C3%A9"
        );
        assert_eq!(
            activation(&arguments, nothing_typed),
            Some(NotificationActivation {
                id: "message 7 & more=é".to_owned(),
                action: None,
                reply: None,
            })
        );
    }

    #[test]
    fn a_click_on_a_button_names_its_action_and_a_reply_brings_what_was_typed() {
        let button = of_action("message-7", "later", None);
        assert_eq!(
            activation(&button, nothing_typed).and_then(|click| click.action),
            Some("later".to_owned())
        );
        let reply = of_action("message-7", "answer", Some("reply-1"));
        let typed = |input: &str| (input == "reply-1").then(|| "on my way".to_owned());
        assert_eq!(
            activation(&reply, typed),
            Some(NotificationActivation {
                id: "message-7".to_owned(),
                action: Some("answer".to_owned()),
                reply: Some("on my way".to_owned()),
            })
        );
        assert_eq!(
            activation(&reply, nothing_typed).and_then(|click| click.reply),
            Some(String::new()),
            "a reply sent empty is an empty reply"
        );
    }

    #[test]
    fn arguments_of_another_toast_tell_nothing() {
        assert_eq!(activation("", nothing_typed), None);
        assert_eq!(activation("action=later", nothing_typed), None);
        assert_eq!(activation("page=settings", nothing_typed), None);
        assert_eq!(activation("inset-notification=%E", nothing_typed), None);
    }
}

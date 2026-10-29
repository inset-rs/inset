//! A notification as a toast's XML, and its tag: plain text, which the system reads.
//!
//! The toast and each of its buttons carry arguments that name the notification and the action
//! (`arguments`), which the system hands back on a click. A reply action is a text box of its own
//! with a send button beside it, the box named after the action's place among the notification's
//! actions, so the click's arguments say which box the reply is in.
//!
//! Plain text over the standard library, the embedder's types and `arguments` alone, so it can be
//! built and tested on any system.

use std::fmt::Write;
use std::time::{Duration, SystemTime};

use inset_embedder::{Notification, NotificationAction};

use super::arguments;

/// The longest tag the system takes, in UTF-16 units: 64 from the Creators Update.
const LONGEST_TAG: usize = 64;

/// The tag a notification's toasts carry: its id, or, for an id longer than a tag may be, a hash
/// of the id the same in every run, so a later post or `remove` names the same toast.
pub(crate) fn tag(id: &str) -> String {
    if id.encode_utf16().count() <= LONGEST_TAG {
        id.to_owned()
    } else {
        format!("inset-{:016x}", fnv1a(id.as_bytes()))
    }
}

/// The toast's XML for `notification`, its image the file at `image_uri`.
pub(crate) fn toast_xml(notification: &Notification, image_uri: Option<&str>) -> String {
    let mut xml = String::new();
    let _ = write!(
        xml,
        r#"<toast launch="{}" activationType="foreground">"#,
        escaped(&arguments::of_notification(&notification.id))
    );
    xml.push_str(&visual(notification, image_uri));
    xml.push_str(&actions(&notification.id, &notification.actions));
    if notification.silent {
        xml.push_str(r#"<audio silent="true"/>"#);
    }
    xml.push_str("</toast>");
    xml
}

/// The title, the body, and the image as the app's logo.
fn visual(notification: &Notification, image_uri: Option<&str>) -> String {
    let mut xml = String::from(r#"<visual><binding template="ToastGeneric">"#);
    let _ = write!(xml, "<text>{}</text>", escaped(&notification.title));
    if let Some(body) = &notification.body {
        let _ = write!(xml, "<text>{}</text>", escaped(body));
    }
    if let Some(uri) = image_uri {
        let _ = write!(
            xml,
            r#"<image placement="appLogoOverride" src="{}"/>"#,
            escaped(uri)
        );
    }
    xml.push_str("</binding></visual>");
    xml
}

/// The text boxes of the reply actions, which the system wants before every button, then a
/// button for each action; nothing for none.
fn actions(id: &str, actions: &[NotificationAction]) -> String {
    if actions.is_empty() {
        return String::new();
    }
    let mut inputs = String::new();
    let mut buttons = String::new();
    for (place, action) in actions.iter().enumerate() {
        let input = action
            .reply_placeholder()
            .map(|placeholder| (input_id(place), placeholder));
        if let Some((input, placeholder)) = &input {
            let _ = write!(
                inputs,
                r#"<input id="{input}" type="text" placeHolderContent="{}"/>"#,
                escaped(placeholder)
            );
        }
        let input = input.map(|(input, _)| input);
        let _ = write!(
            buttons,
            r#"<action content="{}" arguments="{}" activationType="foreground""#,
            escaped(&action.label),
            escaped(&arguments::of_action(id, &action.id, input.as_deref()))
        );
        if let Some(input) = input {
            let _ = write!(buttons, r#" hint-inputId="{input}""#);
        }
        buttons.push_str("/>");
    }
    format!("<actions>{inputs}{buttons}</actions>")
}

/// The id of the text box of the reply action at `place` among the notification's actions.
fn input_id(place: usize) -> String {
    format!("reply-{place}")
}

/// `text` as XML's text or an attribute's value.
fn escaped(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// `at` as a WinRT `DateTime`'s ticks, hundreds of nanoseconds since 1601; `None`, for at once,
/// where `at` is less than a second after `now`, as the system refuses to schedule a time it has
/// passed by the time it is asked.
pub(crate) fn delivery_ticks(at: SystemTime, now: SystemTime) -> Option<i64> {
    let wait = at.duration_since(now).ok()?;
    if wait < Duration::from_secs(1) {
        return None;
    }
    const SECONDS_FROM_1601_TO_1970: u64 = 11_644_473_600;
    let since_1970 = at.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    let since_1601 = since_1970 + Duration::from_secs(SECONDS_FROM_1601_TO_1970);
    i64::try_from(since_1601.as_nanos() / 100).ok()
}

/// FNV-1a, 64 bits: the same in every run and on every build, as `std`'s hasher is not.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use inset_embedder::NotificationReplies;

    use super::*;

    struct Replies;
    impl NotificationReplies for Replies {}

    #[test]
    fn a_notification_is_a_generic_toast_with_its_buttons_and_reply_box() {
        let mut message = Notification::new("message-7", "New <message>");
        message.body = Some("Tom & Jerry said \"hi\"".to_owned());
        message.actions = vec![
            NotificationAction::button("later", "Later"),
            Replies.reply_action("answer", "Reply", "Your answer"),
        ];
        message.silent = true;
        let xml = toast_xml(&message, Some("file:///C:/Temp/inset-notifications/a.png"));
        assert_eq!(
            xml,
            concat!(
                r#"<toast launch="inset-notification=message-7" activationType="foreground">"#,
                r#"<visual><binding template="ToastGeneric">"#,
                "<text>New &lt;message&gt;</text>",
                "<text>Tom &amp; Jerry said &quot;hi&quot;</text>",
                r#"<image placement="appLogoOverride" src="file:///C:/Temp/inset-notifications/a.png"/>"#,
                "</binding></visual>",
                "<actions>",
                r#"<input id="reply-1" type="text" placeHolderContent="Your answer"/>"#,
                r#"<action content="Later" arguments="inset-notification=message-7&amp;action=later" activationType="foreground"/>"#,
                r#"<action content="Reply" arguments="inset-notification=message-7&amp;action=answer&amp;input=reply-1" activationType="foreground" hint-inputId="reply-1"/>"#,
                "</actions>",
                r#"<audio silent="true"/>"#,
                "</toast>"
            )
        );
    }

    #[test]
    fn a_plain_notification_has_no_actions_audio_or_image() {
        let xml = toast_xml(&Notification::new("download", "Downloaded"), None);
        assert_eq!(
            xml,
            concat!(
                r#"<toast launch="inset-notification=download" activationType="foreground">"#,
                r#"<visual><binding template="ToastGeneric"><text>Downloaded</text></binding></visual>"#,
                "</toast>"
            )
        );
    }

    #[test]
    fn a_long_id_is_tagged_by_its_hash_and_a_short_one_by_itself() {
        assert_eq!(tag("message-7"), "message-7");
        let long = "a".repeat(65);
        assert_eq!(tag(&long), tag(&long));
        assert_eq!(tag(&long).len(), "inset-".len() + 16);
        assert_ne!(tag(&long), tag(&"b".repeat(65)));
        assert_eq!(tag(&"é".repeat(64)), "é".repeat(64));
    }

    #[test]
    fn a_time_is_ticks_since_1601_and_one_about_to_pass_is_at_once() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        assert_eq!(delivery_ticks(now, now), None);
        assert_eq!(delivery_ticks(now + Duration::from_millis(999), now), None);
        assert_eq!(
            delivery_ticks(now + Duration::from_secs(60), now),
            Some((11_644_473_600 + 1_060) * 10_000_000)
        );
    }
}

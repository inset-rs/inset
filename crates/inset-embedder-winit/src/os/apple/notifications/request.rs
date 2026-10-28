//! A notification as the center's request: its content, its actions as the category named after
//! them, its image as an attachment and its time as a calendar trigger.
//!
//! A category's id is made from its actions alone, the same in every run, so a notification
//! scheduled in one run finds its category in the next, and the categories the center keeps grow
//! only with the sets of actions the app shows.

use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use inset_embedder::{IconImage, Notification, NotificationAction, NotificationError};
use objc2::rc::Retained;
use objc2_foundation::{
    NSArray, NSCalendar, NSCalendarIdentifierGregorian, NSCalendarUnit, NSDate, NSString,
    NSTimeZone, NSURL, ns_string,
};
use objc2_user_notifications::{
    UNCalendarNotificationTrigger, UNMutableNotificationContent, UNNotificationAction,
    UNNotificationActionOptions, UNNotificationAttachment, UNNotificationCategory,
    UNNotificationCategoryOptions, UNNotificationRequest, UNNotificationSound,
    UNNotificationTrigger, UNTextInputNotificationAction,
};

/// An action brings the app forward, as a click on the notification itself does.
const ACTION_OPTIONS: UNNotificationActionOptions = UNNotificationActionOptions::Foreground;

/// The calendar units a trigger's time is matched in: down to the second, in a calendar and a
/// zone of its own, so the trigger names one moment wherever the user goes.
const TRIGGER_UNITS: NSCalendarUnit = NSCalendarUnit::Year
    .union(NSCalendarUnit::Month)
    .union(NSCalendarUnit::Day)
    .union(NSCalendarUnit::Hour)
    .union(NSCalendarUnit::Minute)
    .union(NSCalendarUnit::Second)
    .union(NSCalendarUnit::Calendar)
    .union(NSCalendarUnit::TimeZone);

static NEXT_IMAGE: AtomicU64 = AtomicU64::new(0);

/// The request that shows `notification` under its id, at `trigger`'s time or at once.
pub(super) fn request(
    notification: &Notification,
    trigger: Option<Retained<UNNotificationTrigger>>,
) -> Result<Retained<UNNotificationRequest>, NotificationError> {
    let content = content(notification)?;
    Ok(
        UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&notification.id),
            &content,
            trigger.as_deref(),
        ),
    )
}

/// What the notification shows: its title, body and image, the system's sound unless it is
/// silent, and its actions' category.
fn content(
    notification: &Notification,
) -> Result<Retained<UNMutableNotificationContent>, NotificationError> {
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(&notification.title));
    if let Some(body) = &notification.body {
        content.setBody(&NSString::from_str(body));
    }
    if !notification.silent {
        content.setSound(Some(&UNNotificationSound::defaultSound()));
    }
    if let Some(category) = category_id(&notification.actions) {
        content.setCategoryIdentifier(&NSString::from_str(&category));
    }
    if let Some(image) = &notification.image {
        content.setAttachments(&NSArray::from_retained_slice(&[attachment(image)?]));
    }
    Ok(content)
}

/// The id of the category that shows `actions`, made from them alone; `None` for no actions.
pub(super) fn category_id(actions: &[NotificationAction]) -> Option<String> {
    if actions.is_empty() {
        return None;
    }
    let mut hash = Fnv1a::default();
    for action in actions {
        hash.write(action.id.as_bytes());
        hash.write(action.label.as_bytes());
        match action.reply_placeholder() {
            Some(placeholder) => {
                hash.write(b"reply");
                hash.write(placeholder.as_bytes());
            }
            None => hash.write(b"button"),
        }
    }
    Some(format!("inset.actions.{:016x}", hash.0))
}

/// The category named `id` that shows `actions`: a button for each, or a reply field for one that
/// asks for a reply, sent with a button of the same label.
pub(super) fn category(
    id: &str,
    actions: &[NotificationAction],
) -> Retained<UNNotificationCategory> {
    let actions: Vec<Retained<UNNotificationAction>> = actions.iter().map(action).collect();
    UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
        &NSString::from_str(id),
        &NSArray::from_retained_slice(&actions),
        &NSArray::new(),
        UNNotificationCategoryOptions::empty(),
    )
}

fn action(action: &NotificationAction) -> Retained<UNNotificationAction> {
    let id = NSString::from_str(&action.id);
    let label = NSString::from_str(&action.label);
    match action.reply_placeholder() {
        Some(placeholder) => Retained::into_super(
            UNTextInputNotificationAction::actionWithIdentifier_title_options_textInputButtonTitle_textInputPlaceholder(
                &id,
                &label,
                ACTION_OPTIONS,
                &label,
                &NSString::from_str(placeholder),
            ),
        ),
        None => UNNotificationAction::actionWithIdentifier_title_options(&id, &label, ACTION_OPTIONS),
    }
}

/// `image` as an attachment: a PNG file of its own, which the center moves into its store when
/// the request is added.
fn attachment(image: &IconImage) -> Result<Retained<UNNotificationAttachment>, NotificationError> {
    let path = written_png(image).map_err(|error| {
        NotificationError::Failed(format!("the image was not written: {error}"))
    })?;
    let attached = NSURL::from_file_path(&path)
        .ok_or_else(|| NotificationError::Failed(format!("{} has no URL", path.display())))
        .and_then(|url| {
            // SAFETY: no options, which the call takes as a dictionary of its own keys.
            unsafe {
                UNNotificationAttachment::attachmentWithIdentifier_URL_options_error(
                    ns_string!("image"),
                    &url,
                    None,
                )
            }
            .map_err(|error| NotificationError::Failed(error.localizedDescription().to_string()))
        });
    if attached.is_err() {
        let _ = fs::remove_file(&path);
    }
    attached
}

/// Writes `image` as a PNG file in the temporary folder, under a name no other image of the
/// process has.
fn written_png(image: &IconImage) -> io::Result<PathBuf> {
    let folder = std::env::temp_dir().join("inset-notifications");
    fs::create_dir_all(&folder)?;
    let path = folder.join(format!(
        "{}-{}.png",
        std::process::id(),
        NEXT_IMAGE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(&path)?),
        image.width,
        image.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&image.rgba)?;
    Ok(path)
}

/// The trigger that fires at `at`, read on a calendar, as the time a clock reads; `None`, for at
/// once, where `at` is less than a second after `now`, as a trigger names whole seconds.
pub(super) fn calendar_trigger(
    at: SystemTime,
    now: SystemTime,
) -> Option<Retained<UNNotificationTrigger>> {
    let wait = at.duration_since(now).ok()?;
    if wait < Duration::from_secs(1) {
        return None;
    }
    let since_1970 = at.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    let whole_second = since_1970.as_secs() + u64::from(since_1970.subsec_nanos() > 0);
    let date = NSDate::dateWithTimeIntervalSince1970(whole_second as f64);
    // SAFETY: a constant Foundation exports for the life of the process.
    let calendar = NSCalendar::calendarWithIdentifier(unsafe { NSCalendarIdentifierGregorian })?;
    calendar.setTimeZone(&NSTimeZone::timeZoneForSecondsFromGMT(0));
    let components = calendar.components_fromDate(TRIGGER_UNITS, &date);
    Some(Retained::into_super(
        UNCalendarNotificationTrigger::triggerWithDateMatchingComponents_repeats(
            &components,
            false,
        ),
    ))
}

/// FNV-1a, 64 bits, over byte strings each closed by a zero byte: the same in every run and on
/// every build, as `std`'s hasher is not.
struct Fnv1a(u64);

impl Default for Fnv1a {
    fn default() -> Fnv1a {
        Fnv1a(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv1a {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes.iter().chain(&[0]) {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use inset_embedder::NotificationReplies;
    use objc2_user_notifications::UNNotificationContent;

    use super::*;

    struct Replies;
    impl NotificationReplies for Replies {}

    fn message() -> Notification {
        let mut message = Notification::new("message-7", "New message");
        message.body = Some("On my way".to_owned());
        message.actions = vec![
            NotificationAction::button("later", "Later"),
            Replies.reply_action("answer", "Reply", "Your answer"),
        ];
        message
    }

    #[test]
    fn a_request_carries_the_id_title_body_sound_and_category() {
        let posted = request(&message(), None).expect("a request");
        assert_eq!(posted.identifier().to_string(), "message-7");
        assert!(posted.trigger().is_none(), "shown at once");
        let content: Retained<UNNotificationContent> = posted.content();
        assert_eq!(content.title().to_string(), "New message");
        assert_eq!(content.body().to_string(), "On my way");
        assert!(content.sound().is_some());
        assert_eq!(
            content.categoryIdentifier().to_string(),
            category_id(&message().actions).expect("a category")
        );
        let mut quiet = Notification::new("quiet", "Synced");
        quiet.silent = true;
        let quiet = request(&quiet, None).expect("a request").content();
        assert!(quiet.sound().is_none());
        assert_eq!(quiet.categoryIdentifier().to_string(), "");
    }

    #[test]
    fn a_category_is_named_by_its_actions_and_shows_a_button_and_a_reply_field() {
        let actions = message().actions;
        let id = category_id(&actions).expect("a category");
        assert_eq!(
            category_id(&actions),
            Some(id.clone()),
            "the same each time"
        );
        assert_ne!(category_id(&actions[..1]), Some(id.clone()));
        assert_eq!(category_id(&[]), None);
        let category = category(&id, &actions);
        let shown = category.actions();
        assert_eq!(shown.count(), 2);
        let button = shown.objectAtIndex(0);
        assert_eq!(button.identifier().to_string(), "later");
        assert!(
            button
                .downcast_ref::<UNTextInputNotificationAction>()
                .is_none()
        );
        let reply = shown.objectAtIndex(1);
        let field = reply
            .downcast_ref::<UNTextInputNotificationAction>()
            .expect("a reply field");
        assert_eq!(field.textInputPlaceholder().to_string(), "Your answer");
        assert_eq!(field.textInputButtonTitle().to_string(), "Reply");
    }

    #[test]
    fn an_image_is_attached_as_a_png_file() {
        let mut pictured = Notification::new("pictured", "Photo");
        pictured.image = Some(IconImage {
            width: 2,
            height: 1,
            rgba: Arc::from(vec![255, 0, 0, 255, 0, 0, 255, 128]),
            template: false,
        });
        let content = request(&pictured, None).expect("a request").content();
        let attachments = content.attachments();
        assert_eq!(attachments.count(), 1);
        let url = attachments.objectAtIndex(0).URL();
        let path = url.to_file_path().expect("a file");
        let decoded = png::Decoder::new(io::BufReader::new(File::open(&path).expect("the file")))
            .read_info()
            .expect("a PNG");
        assert_eq!((decoded.info().width, decoded.info().height), (2, 1));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn a_trigger_fires_at_the_whole_second_and_a_past_time_is_at_once() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let at = now + Duration::from_millis(90_250);
        let trigger = calendar_trigger(at, now).expect("a trigger");
        let trigger = trigger
            .downcast_ref::<UNCalendarNotificationTrigger>()
            .expect("a calendar trigger");
        assert!(!trigger.repeats());
        let fires = named_moment(trigger);
        assert_eq!(fires, 1_800_000_091.0);
        assert!(calendar_trigger(now - Duration::from_secs(5), now).is_none());
        assert!(calendar_trigger(now + Duration::from_millis(400), now).is_none());
    }

    /// The moment `trigger` names, read back from its components.
    fn named_moment(trigger: &UNCalendarNotificationTrigger) -> f64 {
        let components = trigger.dateComponents();
        let calendar = components.calendar().expect("the trigger's calendar");
        calendar
            .dateFromComponents(&components)
            .expect("a date")
            .timeIntervalSince1970()
    }
}
